//! Shutdown Service - graceful server shutdown coordination
//!
//! This service coordinates graceful shutdown of the load balancer,
//! handling signal broadcasting, state tracking, and timeout management.

use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};
use crate::domain::{LoadBalancerError, Result};

/// Shutdown coordinator for graceful server termination
///
/// Coordinates graceful shutdown with configurable timeout and signal broadcasting.
/// Uses Notify for thread-safe async coordination.
#[derive(Debug)]
pub struct ShutdownCoordinator {
    shutdown_signal: Arc<Notify>,
    timeout_duration: Duration,
}

impl ShutdownCoordinator {
    /// Minimum allowed shutdown timeout in seconds
    const MIN_TIMEOUT_SECONDS: u64 = 1;

    /// Create new shutdown coordinator with validated timeout
    ///
    /// Timeout must be at least 1 second.
    pub fn new(timeout_seconds: u64) -> Result<Self> {
        if timeout_seconds < Self::MIN_TIMEOUT_SECONDS {
            return Err(LoadBalancerError::configuration(
                format!(
                    "Shutdown timeout must be at least {} seconds, got {}",
                    Self::MIN_TIMEOUT_SECONDS,
                    timeout_seconds
                )
            ));
        }

        Ok(Self {
            shutdown_signal: Arc::new(Notify::new()),
            timeout_duration: Duration::from_secs(timeout_seconds),
        })
    }

    /// Trigger graceful shutdown
    ///
    /// Notifies all waiting tasks. This method is idempotent.
    pub fn shutdown(&self) {
        self.shutdown_signal.notify_waiters();
        tracing::info!("Shutdown initiated");
    }

    /// Wait for shutdown signal
    ///
    /// Blocks the current task until shutdown is triggered.
    pub async fn wait_for_shutdown(&self) {
        self.shutdown_signal.notified().await;
    }

    /// Wait for shutdown signal with configured timeout
    ///
    /// Returns `true` if shutdown received, `false` if timeout exceeded.
    pub async fn wait_for_shutdown_with_timeout(&self) -> bool {
        match timeout(self.timeout_duration, self.shutdown_signal.notified()).await {
            Ok(_) => true,
            Err(_) => {
                tracing::warn!(
                    timeout_seconds = self.timeout_duration.as_secs(),
                    "Shutdown timeout exceeded"
                );
                false
            }
        }
    }

    /// Get a cloneable shutdown signal for distribution to tasks
    pub fn subscribe(&self) -> ShutdownSignal {
        ShutdownSignal {
            notify: self.shutdown_signal.clone(),
        }
    }
}

/// Cloneable shutdown signal for distribution to tasks
#[derive(Clone)]
pub struct ShutdownSignal {
    notify: Arc<Notify>,
}

impl ShutdownSignal {
    /// Wait for shutdown signal
    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator_creation() {
        let _coordinator = ShutdownCoordinator::new(30).unwrap();
        // Coordinator created successfully
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_validation_min() {
        let result = ShutdownCoordinator::new(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 1 seconds"));
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        use std::sync::Arc;
        let coordinator = Arc::new(ShutdownCoordinator::new(30).unwrap());
        
        // Spawn task to wait for shutdown in background
        let coordinator_clone = coordinator.clone();
        let handle = tokio::task::spawn(async move {
            coordinator_clone.wait_for_shutdown().await;
            "completed"
        });
        
        // Small delay to ensure task is waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Trigger shutdown
        coordinator.shutdown();
        
        // Verify task completes
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            handle
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "completed");
    }
}
