/// Load Balancer Library

/// Domain models and error types  
pub mod domain;

/// Load balancing strategy implementations
pub mod load_balancing_strategy;

/// HTTP routes layer - handles request routing
pub mod routes;

/// Business logic services layer
pub mod services;

/// Shared utility functions
pub mod utils;

// Re-export domain types for convenient access
pub use domain::{LoadBalancerError, Result, StrategyType, ChangeStrategyRequest, StrategyResponse};

/// Type alias for HTTP response bodies used throughout the load balancer
pub type ResponseBody = http_body_util::combinators::BoxBody<
    hyper::body::Bytes, 
    Box<dyn std::error::Error + Send + Sync>
>;