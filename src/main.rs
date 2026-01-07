/// Load Balancer Main Server
/// 
/// This is the main entry point for the load balancer server.
/// It coordinates between the core load balancer functionality and admin API.

use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use hyper::{body::Incoming, service::service_fn, Request, Response};
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo};
use tokio::{net::TcpListener, task::{self, JoinSet}, signal};
use tracing::{error, info, warn};
use color_eyre::Result;

use load_balancer::load_balancing_strategy::LoadBalancingStrategy;
use load_balancer::routes::Router;
use load_balancer::services::{LoadBalancerService, ShutdownCoordinator};
use load_balancer::domain::WorkerUrl;
use load_balancer::ResponseBody;

/// Main request handler using the new Router architecture
/// 
/// This function uses the new Router to handle all incoming requests
/// with proper separation of concerns between routes and services.
async fn handle(
    req: Request<Incoming>,
    router: Arc<Router>,
) -> Result<Response<ResponseBody>, Infallible> {
    match router.handle(req).await {
        Ok(response) => Ok(response),
        Err(domain_error) => Ok(domain_error.into_response()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    
    let config = load_balancer::utils::Config::parse_config()
        .map_err(|e| color_eyre::eyre::eyre!("Configuration error: {}", e))?;
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("load_balancer={},tower_http=info,hyper=info", config.logging.level).into())
        )
        .with_target(false)
        .init();

    info!("🚀 Starting Load Balancer Server");
    info!("📊 Configuration: {}:{}", config.server.host, config.server.port);

    let worker_hosts: Result<Vec<WorkerUrl>, _> = config.workers.hosts
        .iter()
        .filter(|worker| worker.enabled)
        .map(|worker| WorkerUrl::parse(worker.url.clone()))
        .collect();
    
    let worker_hosts = worker_hosts
        .map_err(|e| color_eyre::eyre::eyre!("Invalid worker URL in configuration: {}", e))?;

    if worker_hosts.is_empty() {
        return Err(color_eyre::eyre::eyre!("No enabled workers configured"));
    }

    info!("🔧 Configured worker hosts: {:?}", worker_hosts.iter().map(|w| w.as_str()).collect::<Vec<_>>());

    let strategy_type = load_balancer::domain::StrategyType::from_str(&config.server.strategy)
        .map_err(|e| color_eyre::eyre::eyre!("Invalid strategy in configuration: {}", e))?;

    let strategy = match strategy_type {
        load_balancer::domain::StrategyType::RoundRobin => {
            LoadBalancingStrategy::new_round_robin()
        }
        load_balancer::domain::StrategyType::LeastConnections => {
            LoadBalancingStrategy::new_least_connections(worker_hosts.len())
                .map_err(|e| color_eyre::eyre::eyre!("Failed to create least connections strategy: {}", e))?
        }
    };

    // Create decision thresholds from configuration
    let thresholds = load_balancer::domain::DecisionThresholds {
        high_latency_ms: config.adaptive.high_latency_ms,
        high_error_rate: config.adaptive.high_error_rate,
        min_samples: config.adaptive.min_samples,
        cooldown: std::time::Duration::from_secs(config.adaptive.cooldown_seconds),
    };

    let load_balancer_service = Arc::new(
        LoadBalancerService::with_adaptive_config(
            worker_hosts,
            strategy,
            config.server.adaptive,
            thresholds,
            config.adaptive.ema_alpha,
        ).map_err(|e| color_eyre::eyre::eyre!("Failed to create load balancer service: {}", e))?,
    );

    // Spawn background evaluation task if adaptive mode is enabled
    if load_balancer_service.is_adaptive() {
        info!("🤖 Adaptive load balancing enabled");
        info!("   Evaluation interval: {}s", config.adaptive.evaluation_interval_seconds);
        info!("   Latency threshold: {}ms", config.adaptive.high_latency_ms);
        info!("   Error rate threshold: {:.1}%", config.adaptive.high_error_rate * 100.0);
        info!("   Min samples: {}", config.adaptive.min_samples);
        info!("   Cooldown: {}s", config.adaptive.cooldown_seconds);
        
        let lb_service = load_balancer_service.clone();
        let eval_interval = config.adaptive.evaluation_interval_seconds;
        task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(eval_interval));
            loop {
                interval.tick().await;
                if let Err(e) = lb_service.evaluate_and_adapt().await {
                    tracing::error!("Adaptive evaluation failed: {}", e);
                }
            }
        });
    }

    let router = Arc::new(Router::new(load_balancer_service.clone()));

    let addr: SocketAddr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()
            .map_err(|e| color_eyre::eyre::eyre!("Invalid host in configuration: {}", e))?, 
        config.server.port
    ));

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| color_eyre::eyre::eyre!("Failed to bind TCP listener on {}: {}", addr, e))?;

    info!("🚀 Load balancer listening on http://{}", addr);
    info!("📊 Admin API available at http://{}/admin/strategy", addr);
    info!("🔄 Current strategy: {}", load_balancer_service.get_strategy_name().await);

    // Create shutdown coordinator
    let shutdown_coordinator = Arc::new(
        ShutdownCoordinator::new(config.server.shutdown_timeout_seconds)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to create shutdown coordinator: {}", e))?
    );
    let shutdown_coordinator_clone = shutdown_coordinator.clone();

    // Spawn signal handler task
    task::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for shutdown signal: {}", e);
            return;
        }
        
        info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        shutdown_coordinator_clone.shutdown();
    });

    // JoinSet to track active connections
    let mut connections = JoinSet::new();

    // Main server loop with graceful shutdown
    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, remote_addr) = match result {
                    Ok(connection) => connection,
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                
                let router = router.clone();
                let shutdown_coordinator_clone = shutdown_coordinator.clone();

                connections.spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = service_fn(move |req| {
                        let router = router.clone();
                        async move {
                            handle(req, router).await
                        }
                    });

                    let conn = http1::Builder::new()
                        .serve_connection(io, service)
                        .with_upgrades();
                    
                    // Pin the connection for graceful_shutdown
                    let mut conn = std::pin::pin!(conn);
                    
                    tokio::select! {
                        result = conn.as_mut() => {
                            if let Err(e) = result {
                                warn!("Connection error from {}: {}", remote_addr, e);
                            }
                        }
                        _ = shutdown_coordinator_clone.wait_for_shutdown() => {
                            // Graceful shutdown: let connection finish current request
                            tracing::debug!("Gracefully closing connection from {}", remote_addr);
                            conn.as_mut().graceful_shutdown();
                        }
                    }
                });
            }
            
            // Clean up completed connections while running
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(e) = result {
                    warn!("Connection task panicked: {}", e);
                }
            }
            
            _ = shutdown_coordinator.wait_for_shutdown() => {
                info!("Shutdown signal received, stopping acceptance of new connections");
                break;
            }
        }
    }

    let active_connections = connections.len();
    info!(
        timeout_seconds = config.server.shutdown_timeout_seconds,
        active_connections = active_connections,
        "Waiting for active connections to complete"
    );
    
    // Wait for connections with timeout
    let shutdown_timeout = Duration::from_secs(config.server.shutdown_timeout_seconds);
    let shutdown_deadline = tokio::time::Instant::now() + shutdown_timeout;
    
    let mut completed = 0;
    while !connections.is_empty() {
        tokio::select! {
            Some(result) = connections.join_next() => {
                completed += 1;
                if let Err(e) = result {
                    warn!("Connection task panicked: {}", e);
                }
            }
            _ = tokio::time::sleep_until(shutdown_deadline) => {
                let remaining = connections.len();
                warn!(
                    completed = completed,
                    remaining = remaining,
                    "Shutdown timeout reached, {} connections will be aborted",
                    remaining
                );
                connections.abort_all();
                break;
            }
        }
    }
    
    info!(completed = completed, "All connections closed");
    
    info!("Server shutdown complete");
    Ok(())
}
