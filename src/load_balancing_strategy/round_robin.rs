/// Round Robin Load Balancing Algorithm
/// 
/// Distributes requests sequentially across workers using Rust's type system.

use std::sync::atomic::{AtomicUsize, Ordering};
use crate::domain::LoadBalancerError;
use super::traits::Strategy;

/// Round Robin algorithm distributes requests sequentially across workers
/// 
/// This algorithm cycles through workers in order, ensuring equal distribution
/// over time. It's simple, fast, and works well when all workers have similar
/// capacity and request processing times.
#[derive(Debug)]
pub struct RoundRobin {
    current: AtomicUsize,
}

impl RoundRobin {
    /// Create a new Round Robin algorithm instance
    pub fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for RoundRobin {
    fn select_worker(&self, worker_count: usize) -> Result<usize, LoadBalancerError> {
        self.validate_worker_count(worker_count)?;
        
        let current_worker = self.current.fetch_add(1, Ordering::Relaxed);
        Ok(current_worker % worker_count)
    }

    fn strategy_name(&self) -> &'static str {
        "round_robin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_single_worker() {
        let rr = RoundRobin::new();
        
        // With one worker, should always return 0
        assert_eq!(rr.select_worker(1).unwrap(), 0);
        assert_eq!(rr.select_worker(1).unwrap(), 0);
        assert_eq!(rr.select_worker(1).unwrap(), 0);
    }

    #[test]
    fn test_round_robin_no_workers() {
        let rr = RoundRobin::new();
        
        // Should return error with no workers
        assert!(rr.select_worker(0).is_err());
    }

    #[test]
    fn test_round_robin_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let rr = Arc::new(RoundRobin::new());
        let worker_count = 3;
        let mut handles = vec![];

        // Test concurrent access doesn't cause issues
        for _ in 0..10 {
            let rr_clone = Arc::clone(&rr);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let _ = rr_clone.select_worker(worker_count);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Algorithm should still work after concurrent access
        let result = rr.select_worker(worker_count).unwrap();
        assert!(result < worker_count);
    }
}