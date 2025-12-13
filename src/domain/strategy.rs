/// Strategy domain model with type-safe enums

use serde::{Deserialize, Serialize};
use crate::LoadBalancerError;

/// Load balancing strategy enum (replaces string-based approach)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    RoundRobin,
    LeastConnections,
}

impl StrategyType {
    /// Get strategy name as string for API responses
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyType::RoundRobin => "round_robin",
            StrategyType::LeastConnections => "least_connections", 
        }
    }
    
    /// Parse strategy from string with validation
    pub fn from_str(s: &str) -> Result<Self, LoadBalancerError> {
        match s.to_lowercase().as_str() {
            "round_robin" => Ok(StrategyType::RoundRobin),
            "least_connections" => Ok(StrategyType::LeastConnections),
            _ => Err(LoadBalancerError::invalid_strategy(s)),
        }
    }
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_serialization() {
        let strategy = StrategyType::RoundRobin;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"round_robin\"");
    }

    #[test]
    fn test_strategy_deserialization() {
        let json = "\"least_connections\"";
        let strategy: StrategyType = serde_json::from_str(json).unwrap();
        assert_eq!(strategy, StrategyType::LeastConnections);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(StrategyType::from_str("round_robin").unwrap(), StrategyType::RoundRobin);
        assert_eq!(StrategyType::from_str("least_connections").unwrap(), StrategyType::LeastConnections);
        assert!(StrategyType::from_str("invalid").is_err());
    }
}