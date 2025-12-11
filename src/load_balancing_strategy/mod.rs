/// Load Balancing Strategy Module
/// 
/// This module provides different load balancing algorithms using the strategy pattern.
/// Each algorithm is implemented as a separate type with a common trait interface.

pub mod traits;
pub mod round_robin;
pub mod least_connections;

// Re-export the main types for convenience
pub use traits::Strategy;
pub use round_robin::RoundRobin;
pub use least_connections::LeastConnections;

use crate::LoadBalancerError;

/// Unified enum for managing different algorithms
/// 
/// This provides a single enum interface while maintaining the separate types underneath.
/// Bridges the new trait-based system with the existing enum-based code.
#[derive(Debug)]
pub enum LoadBalancingStrategy {
    RoundRobin(RoundRobin),
    LeastConnections(LeastConnections),
}

impl LoadBalancingStrategy {
    /// Create a new round robin strategy
    pub fn new_round_robin() -> Self {
        Self::RoundRobin(RoundRobin::new())
    }

    /// Create a new least connections strategy
    pub fn new_least_connections(worker_count: usize) -> Result<Self, LoadBalancerError> {
        Ok(Self::LeastConnections(LeastConnections::new(worker_count)?))
    }

    /// Get the strategy name
    pub fn strategy_name(&self) -> &'static str {
        match self {
            Self::RoundRobin(rr) => rr.strategy_name(),
            Self::LeastConnections(lc) => lc.strategy_name(),
        }
    }

    /// Select a worker using the underlying algorithm
    pub fn select_worker(&self, worker_count: usize) -> Result<usize, LoadBalancerError> {
        match self {
            Self::RoundRobin(rr) => rr.select_worker(worker_count),
            Self::LeastConnections(lc) => lc.select_worker(worker_count),
        }
    }

    /// Notify that a connection has started
    pub fn connection_started(&self, worker_index: usize) {
        match self {
            Self::RoundRobin(rr) => rr.connection_started(worker_index),
            Self::LeastConnections(lc) => lc.connection_started(worker_index),
        }
    }

    /// Notify that a connection has ended
    pub fn connection_ended(&self, worker_index: usize) {
        match self {
            Self::RoundRobin(rr) => rr.connection_ended(worker_index),
            Self::LeastConnections(lc) => lc.connection_ended(worker_index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_strategy() {
        let strategy = LoadBalancingStrategy::new_round_robin();
        assert_eq!(strategy.strategy_name(), "round_robin");
        
        // Test basic functionality
        assert_eq!(strategy.select_worker(3).unwrap(), 0);
        assert_eq!(strategy.select_worker(3).unwrap(), 1);
        assert_eq!(strategy.select_worker(3).unwrap(), 2);
        assert_eq!(strategy.select_worker(3).unwrap(), 0);
    }

    #[test]
    fn test_least_connections_strategy() {
        let strategy = LoadBalancingStrategy::new_least_connections(3).unwrap();
        assert_eq!(strategy.strategy_name(), "least_connections");
        
        // Test basic functionality
        let worker = strategy.select_worker(3).unwrap();
        assert!(worker < 3);
        
        // Test connection tracking
        strategy.connection_started(0);
        strategy.connection_ended(0);
    }

    #[test]
    fn test_strategy_polymorphism() {
        let strategies: Vec<LoadBalancingStrategy> = vec![
            LoadBalancingStrategy::new_round_robin(),
            LoadBalancingStrategy::new_least_connections(3).unwrap(),
        ];

        for strategy in strategies {
            // All strategies should implement the same interface
            let worker = strategy.select_worker(3).unwrap();
            assert!(worker < 3);
            
            // Connection tracking should work for all (no-op for round robin)
            strategy.connection_started(worker);
            strategy.connection_ended(worker);
            
            // All should have names
            assert!(!strategy.strategy_name().is_empty());
        }
    }
}