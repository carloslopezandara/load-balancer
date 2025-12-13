/// Least Connections Load Balancing Algorithm
/// 
/// Routes requests to the worker with fewest active connections.

use std::sync::atomic::{AtomicUsize, Ordering};
use crate::domain::LoadBalancerError;
use super::traits::Strategy;

/// Least Connections algorithm routes requests to the worker with fewest active connections
/// 
/// This algorithm tracks active connections per worker and routes new requests
/// to the worker currently handling the fewest connections. This provides better
/// load distribution when request processing times vary significantly.
#[derive(Debug)]
pub struct LeastConnections {
    connections: Vec<AtomicUsize>,
}

impl LeastConnections {
    /// Create a new Least Connections algorithm instance
    /// 
    /// # Arguments
    /// * `worker_count` - Number of workers to track connections for
    /// 
    /// # Returns
    /// Result with the new instance or an error if worker_count is 0
    pub fn new(worker_count: usize) -> Result<Self, LoadBalancerError> {
        if worker_count == 0 {
            return Err(LoadBalancerError::configuration(
                "Cannot create least connections strategy with 0 workers"
            ));
        }
        
        let connections = (0..worker_count)
            .map(|_| AtomicUsize::new(0))
            .collect();
            
        Ok(Self { connections })
    }
}

impl Strategy for LeastConnections {
    fn select_worker(&self, worker_count: usize) -> Result<usize, LoadBalancerError> {
        self.validate_worker_count(worker_count)?;
        
        if self.connections.is_empty() {
            return Err(LoadBalancerError::configuration(
                "Least connections strategy has no connection counters configured"
            ));
        }

        let mut least_index = 0;
        let mut least_connections = usize::MAX;
        
        for (i, conn) in self.connections.iter().enumerate() {
            let conn_count = conn.load(Ordering::Relaxed);
            if conn_count < least_connections {
                least_connections = conn_count;
                least_index = i;
            }
        }

        Ok(least_index)
    }

    fn connection_started(&self, worker_index: usize) {
        if let Some(conn) = self.connections.get(worker_index) {
            conn.fetch_add(1, Ordering::Relaxed);
        } else {
            tracing::warn!(
                "connection_started called with invalid worker_index: {} (max: {})", 
                worker_index, 
                self.connections.len()
            );
        }
    }

    fn connection_ended(&self, worker_index: usize) {
        if let Some(conn) = self.connections.get(worker_index) {
            let update_result = conn.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                if current > 0 {
                    Some(current - 1)
                } else {
                    None // Don't update if already 0
                }
            });
            
            if update_result.is_err() {
                tracing::warn!(
                    "Attempted to decrement connection count for worker {} but count was already 0", 
                    worker_index
                );
            }
        } else {
            tracing::warn!(
                "connection_ended called with invalid worker_index: {} (max: {})", 
                worker_index, 
                self.connections.len()
            );
        }
    }

    fn strategy_name(&self) -> &'static str {
        "least_connections"
    }

    fn validate_worker_count(&self, worker_count: usize) -> Result<(), LoadBalancerError> {
        if worker_count == 0 {
            return Err(LoadBalancerError::NoWorkersAvailable);
        }
        
        if worker_count != self.connections.len() {
            return Err(LoadBalancerError::configuration(
                format!(
                    "Worker count mismatch: algorithm configured for {} workers but {} requested",
                    self.connections.len(),
                    worker_count
                )
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_least_connections_selection() {
        let lc = LeastConnections::new(3).unwrap();
        
        // Initially should select worker 0 (all have 0 connections)
        assert_eq!(lc.select_worker(3).unwrap(), 0);
        
        // Simulate connection to worker 0
        lc.connection_started(0);
        
        // Now should select worker 1 (has 0, worker 0 has 1)
        assert_eq!(lc.select_worker(3).unwrap(), 1);
        
        // Add connection to worker 1
        lc.connection_started(1);
        
        // Should select worker 2 (has 0, others have 1)
        assert_eq!(lc.select_worker(3).unwrap(), 2);
    }

    #[test]
    fn test_worker_count_validation() {
        let lc = LeastConnections::new(3).unwrap();
        
        assert!(lc.validate_worker_count(0).is_err());
        assert!(lc.validate_worker_count(2).is_err()); // Mismatch
        assert!(lc.validate_worker_count(3).is_ok());  // Match
        assert!(lc.validate_worker_count(4).is_err()); // Mismatch
    }
}