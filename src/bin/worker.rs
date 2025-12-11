/// Worker Binary for Load Balancer Testing
/// 
/// This binary provides a simple HTTP worker server that can be used to test
/// the load balancer's functionality. It responds to HTTP requests with messages
/// that include the worker's port number, making it easy to verify load distribution.
use std::{convert::Infallible, net::SocketAddr, time::Duration};

use clap::Parser;
use hyper::{
    body::Incoming,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use tokio::{net::TcpListener, task};
use tracing::{error, info};

// Import the load balancer's HTTP utilities for consistent responses
use load_balancer::utils::{create_json_response_with_status, create_fallback_error_response};
use load_balancer::domain::{WorkerHealthResponse, WorkerResponse};
use load_balancer::{ResponseBody, LoadBalancerError};

/// Worker server for load balancer testing
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port number to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

/// Worker request handler that simulates different processing times
/// 
/// This handler processes incoming HTTP requests and returns responses
/// that identify which worker handled the request. Different endpoints
/// simulate different processing times to test load balancing behavior.
async fn worker_handler(req: Request<Incoming>, port: u16) -> Result<Response<ResponseBody>, LoadBalancerError> {
    let path = req.uri().path();
    match path {
        // Health check endpoint - returns JSON status
        "/health" => {
                let health_response = WorkerHealthResponse::healthy(port);
                Ok(create_json_response_with_status(&health_response, hyper::StatusCode::OK).unwrap_or_else(|e| {
                    tracing::error!("Failed to serialize health response: {}", e);
                    create_fallback_error_response()
                }))
        },
        // Work simulation endpoint - short processing delay
        "/work" => {
                let message = format!(
                    "worker on port {} is processing {} {}",
                    port,
                    req.method(),
                    req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
                );

                // Simulate fast work processing
                tokio::time::sleep(Duration::from_millis(10)).await;

                let work_response = WorkerResponse::new(message, port);
                Ok(create_json_response_with_status(&work_response, hyper::StatusCode::OK).unwrap_or_else(|e| {
                    tracing::error!("Failed to serialize work response: {}", e);
                    create_fallback_error_response()
                }))
        },
        // Default endpoint - simulates longer processing time
        _ => {
                let message = format!(
                    "worker on port {} received {} {}",
                    port,
                    req.method(),
                    req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
                );

                // Simulate slower request processing for load balancing demonstrations
                tokio::time::sleep(Duration::from_secs(1)).await;

                let default_response = WorkerResponse::new(message, port);
                Ok(create_json_response_with_status(&default_response, hyper::StatusCode::OK).unwrap_or_else(|e| {
                    tracing::error!("Failed to serialize default response: {}", e);
                    create_fallback_error_response()
                }))
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "worker=info".into())
        )
        .with_target(false)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    let host_ip = args.host.parse::<std::net::IpAddr>()
        .unwrap_or_else(|e| {
            tracing::warn!("Invalid host IP '{}': {}, falling back to 127.0.0.1", args.host, e);
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        });
    let addr = SocketAddr::from((host_ip, args.port));
    
    info!("🔧 Worker starting on http://{}", addr);

    // Bind to the specified address
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind worker to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    info!("✅ Worker listening on http://{}", addr);

    // Accept and handle incoming connections
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(connection) => connection,
            Err(e) => {
                error!("Worker failed to accept connection: {}", e);
                continue;
            }
        };

        // Spawn a task for each connection to handle it concurrently
        let worker_port = args.port;
        task::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| async move {
                worker_handler(req, worker_port).await
                    .or_else(|e| {
                        error!("Worker request error: {}", e);
                        Ok::<_, Infallible>(e.into_response())
                    })
            });
            let builder = Builder::new(TokioExecutor::new());

            if let Err(err) = builder.serve_connection(io, service).await {
                error!("Worker connection error: {}", err);
            }
        });
    }
}
