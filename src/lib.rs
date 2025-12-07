/// Load Balancer Library

/// Core load balancer functionality
pub mod load_balancer;

/// Load balancing strategy implementations  
pub mod strategy;

/// Administrative API for runtime management
pub mod admin;

/// Shared utility functions
pub mod utils;

// Re-export HTTP utilities for use across the entire codebase
pub use utils::http_utils;

/// Re-export main types for convenient access
pub use load_balancer::LoadBalancer;
pub use strategy::LoadBalancingStrategy;

/// Type alias for HTTP response bodies used throughout the load balancer
pub type ResponseBody = http_body_util::combinators::BoxBody<
    hyper::body::Bytes, 
    Box<dyn std::error::Error + Send + Sync>
>;