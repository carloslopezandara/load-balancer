/// Load Balancer Main Server
/// 
/// This is the main entry point for the load balancer server.
/// It coordinates between the core load balancer functionality and admin API.

use std::{net::SocketAddr, sync::Arc};
use hyper::{body::Incoming, service::service_fn, Request, Response};
use hyper::server::conn::http1;
use hyper_util::client::legacy::Error as ClientError;
use hyper_util::rt::{TokioIo};
use tokio::{net::TcpListener, task};

use load_balancer::LoadBalancer;
use load_balancer::strategy::LoadBalancingStrategy;
use load_balancer::admin::handle_admin;
use load_balancer::ResponseBody;

/// Main request handler that routes between admin endpoints and worker forwarding
/// 
/// This function acts as the main router, determining whether requests should go to:
/// - Admin API endpoints (paths starting with /admin)
/// - Backend workers (all other paths)
async fn handle(
    req: Request<Incoming>,
    load_balancer: Arc<LoadBalancer>,
) -> Result<Response<ResponseBody>, ClientError> {
    let path = req.uri().path();
    
    // Route admin requests to admin handler, everything else to workers
    if path.starts_with("/admin") {
        handle_admin(req, load_balancer).await
    } else {
        load_balancer.forward_request(req).await
    }
}

#[tokio::main]
async fn main() {
    // Configure backend workers (Manually specified for now)
    let worker_hosts = vec![
        "http://localhost:3000".to_string(),
        "http://localhost:3001".to_string(),
    ];

    // Initialize with least connections strategy
    let strategy = LoadBalancingStrategy::new_least_connections(worker_hosts.len());

    // Create the load balancer instance
    let load_balancer = Arc::new(
        LoadBalancer::new(worker_hosts, strategy).expect("failed to create load balancer"),
    );

    // Configure server address
    let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 1337));

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("🚀 Load balancer listening on http://{}", addr);
    println!("📊 Admin API available at http://{}/admin/strategy", addr);
    println!("🔄 Current strategy: {}", load_balancer.get_strategy_name().await);

    // Main server loop
    loop {
        let (stream, _) = listener.accept().await.expect("failed to accept");
        let load_balancer = load_balancer.clone();

        task::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| handle(req, load_balancer.clone()));

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("error: {}", e);
            }
        });
    }
}
