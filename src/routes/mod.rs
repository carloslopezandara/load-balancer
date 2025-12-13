//! Routes module - handles HTTP routing and request/response mapping
//! 
//! This module contains all HTTP route definitions and maps incoming requests
//! to appropriate service layer functions. It follows the separation of concerns
//! principle by keeping HTTP-specific logic separate from business logic.

mod admin;
mod proxy;

pub use admin::AdminRoutes;
pub use proxy::ProxyRoutes;

use hyper::{Request, Response};
use hyper::body::Incoming;
use std::sync::Arc;
use crate::domain::Result;
use crate::services::{LoadBalancerService, AdminService};
use crate::ResponseBody;

/// Main router that handles all incoming HTTP requests
/// 
/// This struct coordinates routing between different route handlers
/// and provides a unified interface for request handling.
pub struct Router {
    admin_routes: AdminRoutes,
    proxy_routes: ProxyRoutes,
}

impl Router {
    /// Create a new router with the provided services
    pub fn new(load_balancer_service: Arc<LoadBalancerService>) -> Self {
        let admin_service = AdminService::new(load_balancer_service.clone());
        
        Self {
            admin_routes: AdminRoutes::new(admin_service),
            proxy_routes: ProxyRoutes::new(load_balancer_service),
        }
    }

    /// Handle incoming HTTP request by routing to appropriate handler
    /// 
    /// This is the main entry point for all HTTP requests. It determines
    /// the appropriate route handler based on the request path and method.
    pub async fn handle(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ResponseBody>> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        // Log incoming request
        tracing::info!("Incoming request: {} {}", method, path);

        // Route to appropriate handler based on path prefix
        let result = if path.starts_with("/admin") {
            tracing::info!("Routing to admin routes");
            self.admin_routes.handle(req).await
        } else {
            tracing::info!("Routing to proxy (load balancer)");
            self.proxy_routes.forward_request(req).await
        };

        // Log result
        match &result {
            Ok(_) => tracing::info!("Request completed successfully: {} {}", method, path),
            Err(e) => tracing::error!("Request failed: {} {} - Error: {}", method, path, e),
        }

        result
    }
}
