use std::{str::FromStr, sync::Arc};
use hyper::{body::Incoming, Request, Response, Uri};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::{Client, Error as ClientError};
use hyper_util::rt::TokioExecutor;
use http_body_util::BodyExt;
use tokio::sync::RwLock;

use crate::strategy::LoadBalancingStrategy;
use crate::ResponseBody;

/// Core load balancer that distributes HTTP requests across backend workers
/// 
/// The LoadBalancer maintains a pool of worker hosts and uses pluggable strategies
/// to determine how requests are distributed. It supports runtime strategy changes
/// and provides thread-safe operation in concurrent environments.
pub struct LoadBalancer {
    /// HTTP client for forwarding requests to workers
    client: Client<HttpConnector, Incoming>,
    /// List of backend worker URLs
    worker_hosts: Vec<String>,
    /// Current load balancing strategy (protected by RwLock for thread safety)
    strategy: Arc<RwLock<LoadBalancingStrategy>>,
}

impl LoadBalancer {
    /// Creates a new LoadBalancer instance
    pub fn new(worker_hosts: Vec<String>, strategy: LoadBalancingStrategy) -> Result<Self, String> {
        if worker_hosts.is_empty() {
            return Err("No worker hosts provided".into());
        }

        let connector = HttpConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(connector);

        Ok(LoadBalancer {
            client,
            worker_hosts,
            strategy: Arc::new(RwLock::new(strategy)),
        })
    }

    /// Forwards an HTTP request to an appropriate backend worker
    /// 
    /// This method:
    /// 1. Selects a worker using the current load balancing strategy
    /// 2. Forwards the request while preserving headers and body
    /// 3. Tracks connections for strategies that need it (e.g., LeastConnections)
    /// 4. Returns the worker's response
    pub async fn forward_request(&self, req: Request<Incoming>) -> Result<Response<ResponseBody>, ClientError> {
        // Select a worker using the load balancing strategy
        let (worker_index, mut worker_uri) = self.get_worker().await;

        // Notify that a connection has started
        let strategy = self.strategy.read().await;
        strategy.connection_started(worker_index);

        // Extract the path and query from the original request
        if let Some(path_and_query) = req.uri().path_and_query() {
            worker_uri.push_str(path_and_query.as_str());
        }

        let new_uri = Uri::from_str(&worker_uri).unwrap();
        let headers = req.headers().clone();

        // Clone the original request's headers and method
        let mut new_req = Request::builder()
            .method(req.method())
            .uri(new_uri)
            .body(req.into_body())
            .expect("request builder");

        for (key, value) in headers.iter() {
            new_req.headers_mut().insert(key, value.clone());
        }

        let result = self.client.request(new_req).await;
        // Notify that the connection has ended when the response future completes
        strategy.connection_ended(worker_index);
        
        // Convert the worker's response to our consistent ResponseBody type
        match result {
            Ok(response) => {
                let (parts, body) = response.into_parts();
                let boxed_body = body.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>).boxed();
                Ok(Response::from_parts(parts, boxed_body))
            }
            Err(e) => Err(e),
        }
    }

    /// Selects the next worker based on the current load balancing strategy
    async fn get_worker(&self) -> (usize, String) {
        let strategy = self.strategy.read().await;
        let index = strategy.select_worker(self.worker_hosts.len());
        (index, self.worker_hosts[index].clone())
    }

    /// Returns the current strategy name as a string for API responses
    pub async fn get_strategy_name(&self) -> String {
        let strategy = self.strategy.read().await;
        match *strategy {
            LoadBalancingStrategy::RoundRobin { .. } => "round_robin".to_string(),
            LoadBalancingStrategy::LeastConnections { .. } => "least_connections".to_string(),
        }
    }

    /// Changes the load balancing strategy at runtime
    /// 
    /// This operation is thread-safe and will affect all subsequent requests.
    /// Any ongoing requests will complete with the previous strategy.
    pub async fn set_strategy(&self, new_strategy: LoadBalancingStrategy) {
        let mut strategy = self.strategy.write().await;
        *strategy = new_strategy;
    }

    /// Gets the current worker host list
    pub fn worker_hosts(&self) -> &Vec<String> {
        &self.worker_hosts
    }
}