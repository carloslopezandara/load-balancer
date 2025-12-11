//! Services module - contains business logic layer
//! 
//! This module implements the business logic for the load balancer,
//! separated from HTTP routing concerns. Each service focuses on a
//! specific domain area and can be used by multiple route handlers.

// Module declarations (auth-service pattern)
mod admin;
mod load_balancer;
mod metrics;

// Re-export service types
pub use admin::AdminService;
pub use load_balancer::LoadBalancerService;
pub use metrics::MetricsCollector;
