/// Worker Binary for Load Balancer Testing
/// 
/// This binary provides a simple HTTP worker server that can be used to test
/// the load balancer's functionality. It responds to HTTP requests with messages
/// that include the worker's port number, making it easy to verify load distribution.
use std::{convert::Infallible, env, net::SocketAddr, time::Duration};

use hyper::{
    body::Incoming,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use tokio::{net::TcpListener, task};

// Import the load balancer's HTTP utilities for consistent responses
use load_balancer::utils::{create_json_response, WorkerHealthResponse, WorkerResponse};
use load_balancer::ResponseBody;

/// HTTP request handler for the worker server
/// 
/// This handler processes incoming HTTP requests and returns responses
/// that identify which worker handled the request. Different endpoints
/// simulate different processing times to test load balancing behavior.
async fn worker_handler(req: Request<Incoming>, port: u16) -> Result<Response<ResponseBody>, Infallible> {
    let path = req.uri().path();
    match path {
        // Health check endpoint - returns JSON status
        "/health" => {
                let health_response = WorkerHealthResponse {
                    status: "healthy".to_string(),
                    port,
                };
                Ok(create_json_response(&health_response))
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

                let work_response = WorkerResponse {
                    message,
                    port,
                };
                Ok(create_json_response(&work_response))
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

                let default_response = WorkerResponse {
                    message,
                    port,
                };
                Ok(create_json_response(&default_response))
        }
    }
}

#[tokio::main]
async fn main() {
    // Parse port from command line args, environment variable, or use default
    let port = env::args()
        .nth(1)
        .and_then(|port| port.parse().ok())
        .or_else(|| env::var("PORT").ok().and_then(|port| port.parse().ok()))
        .unwrap_or(3000);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("worker listening on http://{}", addr);

    // Bind to the specified address
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind worker port");

    // Accept and handle incoming connections
    loop {
        let (stream, _) = listener.accept().await.expect("failed to accept");

        // Spawn a task for each connection to handle it concurrently
        task::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| worker_handler(req, port));
            let builder = Builder::new(TokioExecutor::new());

            if let Err(err) = builder.serve_connection(io, service).await {
                eprintln!("worker connection error: {err}");
            }
        });
    }
}
