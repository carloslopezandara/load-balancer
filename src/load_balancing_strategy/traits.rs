/// Load Balancing Strategy Trait
/// 
/// Defines the common interface for all load balancing strategies.

use crate::domain::LoadBalancerError;

/// Common interface for all load balancing strategies
/// 
/// This trait defines the contract that all load balancing strategies must implement.
/// It provides a clean abstraction over different algorithms while maintaining
/// type safety and consistent error handling.
pub trait Strategy {
    /// Select the next worker based on the algorithm's logic
    /// 
    /// Returns the index of the selected worker or an error if no workers
    /// are available or the configuration is invalid.
    fn select_worker(&self, worker_count: usize) -> Result<usize, LoadBalancerError>;

    /// Notify the algorithm that a connection has started
    /// 
    /// This is used by algorithms that track connection state,
    /// such as least connections. Round robin can ignore this.
    fn connection_started(&self, worker_index: usize) {
        // Default implementation does nothing
        // Algorithms that need connection tracking will override this
        let _ = worker_index; // Avoid unused parameter warning
    }

    /// Notify the algorithm that a connection has ended
    /// 
    /// This is used by algorithms that track connection state,
    /// such as least connections. Round robin can ignore this.
    fn connection_ended(&self, worker_index: usize) {
        // Default implementation does nothing
        // Algorithms that need connection tracking will override this
        let _ = worker_index; // Avoid unused parameter warning
    }

    /// Get the strategy name for debugging and logging
    fn strategy_name(&self) -> &'static str;

    /// Validate the algorithm configuration against worker count
    /// 
    /// Some algorithms may need specific validation logic.
    /// Default implementation accepts any positive worker count.
    fn validate_worker_count(&self, worker_count: usize) -> Result<(), LoadBalancerError> {
        if worker_count == 0 {
            Err(LoadBalancerError::NoWorkersAvailable)
        } else {
            Ok(())
        }
    }
}