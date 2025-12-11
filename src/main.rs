/// Load Balancer Main Server
/// 
/// This is the main entry point for the load balancer server.
/// It coordinates between the core load balancer functionality and admin API.

use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use hyper::{body::Incoming, service::service_fn, Request, Response};
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo};
use tokio::{net::TcpListener, task};
use tracing::{error, info, warn};
use color_eyre::Result;

use load_balancer::load_balancing_strategy::LoadBalancingStrategy;
use load_balancer::routes::Router;
use load_balancer::services::LoadBalancerService;
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

    let load_balancer_service = Arc::new(
        LoadBalancerService::with_adaptive(worker_hosts, strategy, config.server.adaptive)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to create load balancer service: {}", e))?,
    );

    // Spawn background evaluation task if adaptive mode is enabled
    if load_balancer_service.is_adaptive() {
        let lb_service = load_balancer_service.clone();
        task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if let Err(e) = lb_service.evaluate_and_adapt().await {
                    tracing::error!("Adaptive evaluation failed: {}", e);
                }
            }
        });
        info!("🤖 Adaptive load balancing enabled");
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

    // Main server loop
    loop {
        let (stream, remote_addr) = match listener.accept().await {
            Ok(connection) => connection,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };
        
        let router = router.clone();

        task::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                let router = router.clone();
                async move {
                    handle(req, router).await
                }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                warn!("Connection error from {}: {}", remote_addr, e);
            }
        });
    }
}
