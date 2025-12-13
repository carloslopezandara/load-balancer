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