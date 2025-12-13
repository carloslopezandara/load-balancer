//! Admin routes - handles administrative HTTP endpoints
//! 
//! This module defines all administrative routes and delegates business logic
//! to the services layer. It focuses purely on HTTP request/response handling.

use hyper::{Request, Response, Method};
use hyper::body::Incoming;
use crate::domain::{LoadBalancerError, Result};
use crate::services::AdminService;
use crate::ResponseBody;

/// Admin routes handler
/// 
/// Handles administrative endpoints for strategy management.
pub struct AdminRoutes {
    admin_service: AdminService,
}

impl AdminRoutes {
    /// Create new admin routes handler
    pub fn new(admin_service: AdminService) -> Self {
        Self {
            admin_service,
        }
    }

    /// Handle admin HTTP requests
    /// 
    /// Routes admin requests to appropriate service methods based on
    /// HTTP method and path patterns.
    pub async fn handle(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ResponseBody>> {
        let method = req.method();
        let path = req.uri().path();

        match (method, path) {
            (&Method::GET, "/admin/strategy") => {
                self.admin_service.get_current_strategy().await
            }
            (&Method::POST, "/admin/strategy") => {
                self.admin_service.change_strategy(req).await
            }
            _ => {
                tracing::debug!("Request to unknown path: {} {}", method, path);
                Err(LoadBalancerError::not_found("Not found"))
            }
        }
    }
}
