/// Domain Module
/// 
/// Central domain models and types for the load balancer.
/// Following auth-service pattern: explicit mod declarations + pub use re-exports.

// Module declarations
mod admin;
mod decision;
mod error;
mod metrics;
mod request_context;
mod strategy;
mod worker;

// Re-export all public types for convenience
pub use admin::{ChangeStrategyRequest, ChangeStrategyResponse, StrategyResponse, MetricsResponse, WorkerMetricsResponse, DecisionStatusResponse};
pub use decision::{Decision, DecisionThresholds, SwitchReason};
pub use error::LoadBalancerError;
pub use metrics::WorkerMetrics;
pub use request_context::RequestContext;
pub use strategy::StrategyType;
pub use worker::{WorkerResponse, WorkerUrl};

/// Domain Result type alias for convenience
pub type Result<T> = std::result::Result<T, LoadBalancerError>;