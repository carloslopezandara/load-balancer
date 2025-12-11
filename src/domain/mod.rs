/// Domain Module
/// 
/// Central domain models and types for the load balancer.
/// Following auth-service pattern: explicit mod declarations + pub use re-exports.

// Module declarations
mod admin;
mod decision;
mod error;
mod metrics;
mod strategy;
mod worker;

// Re-export all public types for convenience
pub use admin::{ChangeStrategyRequest, ChangeStrategyResponse, StrategyResponse, MetricsResponse, WorkerMetricsResponse};
pub use decision::{Decision, DecisionThresholds, SwitchReason};
pub use error::LoadBalancerError;
pub use metrics::WorkerMetrics;
pub use strategy::StrategyType;
pub use worker::{WorkerHealthResponse, WorkerResponse, WorkerUrl};

/// Domain Result type alias for convenience
pub type Result<T> = std::result::Result<T, LoadBalancerError>;