//! Proxy routes - handles request forwarding to backend workers
//! 
//! This module handles HTTP request forwarding to backend workers
//! using the load balancer service for worker selection.

use hyper::{Request, Response, Uri};
use hyper::body::Incoming;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::BodyExt;
use std::{str::FromStr, sync::Arc, time::Instant};
use crate::domain::{LoadBalancerError, RequestContext, Result};
use crate::services::LoadBalancerService;
use crate::ResponseBody;

/// Proxy routes handler for request forwarding
/// 
/// Handles forwarding HTTP requests to backend workers
/// using the load balancer service for worker selection.
pub struct ProxyRoutes {
    load_balancer_service: Arc<LoadBalancerService>,
    http_client: Client<HttpConnector, Incoming>,
}

impl ProxyRoutes {
    /// Create new proxy routes handler
    pub fn new(load_balancer_service: Arc<LoadBalancerService>) -> Self {
        let connector = HttpConnector::new();
        let http_client = Client::builder(TokioExecutor::new()).build(connector);
        
        Self {
            load_balancer_service,
            http_client,
        }
    }

    /// Forward HTTP request to a backend worker
    /// 
    /// This method:
    /// 1. Selects a worker using the load balancing service
    /// 2. Forwards the request while preserving headers and body
    /// 3. Tracks connections for strategies that need it
    /// 4. Records metrics for adaptive load balancing
    /// 5. Returns the worker's response
    pub async fn forward_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ResponseBody>> {
        let ctx = RequestContext::new(
            req.method().to_string(),
            req.uri().path().to_string(),
        );
        
        tracing::info!(
            request_id = %ctx.request_id,
            method = %ctx.method,
            path = %ctx.path,
            "Processing request"
        );
        
        let start = Instant::now();
        
        let (worker_index, worker_url) = self.load_balancer_service.select_worker().await?;
        
        tracing::debug!(
            request_id = %ctx.request_id,
            worker_index = worker_index,
            worker_url = %worker_url.as_str(),
            "Selected worker"
        );

        self.load_balancer_service.connection_started(worker_index).await;

        let worker_uri = if let Some(path_and_query) = req.uri().path_and_query() {
            format!("{}{}", worker_url.as_str(), path_and_query.as_str())
        } else {
            tracing::debug!("Request has no path_and_query, forwarding to worker root");
            format!("{}/", worker_url.as_str())
        };

        let new_uri = Uri::from_str(&worker_uri)
            .map_err(|e| LoadBalancerError::internal(format!("Invalid worker URI {}: {}", worker_uri, e)))?;
        
        let headers = req.headers().clone();
        let method = req.method().clone();

        let mut new_req = Request::builder()
            .method(method)
            .uri(new_uri)
            .body(req.into_body())
            .map_err(|e| LoadBalancerError::internal(format!("Failed to build request: {}", e)))?;

        *new_req.headers_mut() = headers;

        let result = self.http_client.request(new_req).await;
        
        self.load_balancer_service.connection_ended(worker_index).await;
        
        let duration = start.elapsed();
        
        match result {
            Ok(response) => {
                if response.status().is_success() {
                    self.load_balancer_service
                        .metrics_collector()
                        .record_success(worker_index, duration);
                    
                    tracing::info!(
                        request_id = %ctx.request_id,
                        worker_index = worker_index,
                        status = response.status().as_u16(),
                        duration_ms = duration.as_millis(),
                        "Request completed successfully"
                    );
                } else {
                    self.load_balancer_service
                        .metrics_collector()
                        .record_error(worker_index, duration);
                    
                    tracing::warn!(
                        request_id = %ctx.request_id,
                        worker_index = worker_index,
                        status = response.status().as_u16(),
                        duration_ms = duration.as_millis(),
                        "Request failed"
                    );
                }
                
                let (parts, body) = response.into_parts();
                let boxed_body = body.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>).boxed();
                Ok(Response::from_parts(parts, boxed_body))
            }
            Err(e) => {
                self.load_balancer_service
                    .metrics_collector()
                    .record_error(worker_index, duration);
                
                tracing::error!(
                    request_id = %ctx.request_id,
                    worker_index = worker_index,
                    worker_url = %worker_uri,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Request failed"
                );
                
                Err(LoadBalancerError::http_client(&worker_uri, e))
            }
        }
    }
}
