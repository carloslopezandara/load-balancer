/// Admin Domain Models
/// 
/// DTOs (Data Transfer Objects) for administrative API operations.
/// These structures define the contract for admin endpoints.

use serde::{Deserialize, Serialize};
use super::strategy::StrategyType;

/// Request to change load balancing strategy
#[derive(Deserialize, Debug)]
pub struct ChangeStrategyRequest {
    pub strategy: StrategyType,
}

/// Response showing current strategy
#[derive(Serialize, Debug)]
pub struct StrategyResponse {
    pub current_strategy: StrategyType,
}

impl StrategyResponse {
    pub fn new(strategy: &StrategyType) -> Self {
        Self {
            current_strategy: strategy.clone(),
        }
    }
}

/// Response after successful strategy change
#[derive(Serialize, Debug)]
pub struct ChangeStrategyResponse {
    pub message: String,
    pub strategy: StrategyType,
}

impl ChangeStrategyResponse {
    pub fn new(strategy: &StrategyType) -> Self {
        Self {
            message: format!("Strategy changed to {}", strategy),
            strategy: strategy.clone(),
        }
    }
}

/// Metrics for a single worker
#[derive(Serialize, Debug)]
pub struct WorkerMetricsResponse {
    pub worker_url: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub error_rate: f64,
}

/// Response containing metrics for all workers
#[derive(Serialize, Debug)]
pub struct MetricsResponse {
    pub workers: Vec<WorkerMetricsResponse>,
    pub total_requests: u64,
    pub overall_success_rate: f64,
}

/// Response showing adaptive decision engine status
#[derive(Serialize, Debug)]
pub struct DecisionStatusResponse {
    pub adaptive_enabled: bool,
    pub current_strategy: StrategyType,
    pub last_evaluation: Option<String>,
    pub can_switch: bool,
    pub cooldown_remaining_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_strategy_request_serialization() {
        let json = r#"{"strategy":"round_robin"}"#;
        let request: ChangeStrategyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.strategy, StrategyType::RoundRobin);
    }
}