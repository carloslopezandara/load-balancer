//! Shutdown Service - graceful server shutdown coordination
//!
//! This service coordinates graceful shutdown of the load balancer,
//! handling signal broadcasting, state tracking, and timeout management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};
use crate::domain::{LoadBalancerError, Result};

/// Shutdown coordinator for graceful server termination
///
/// Coordinates graceful shutdown with configurable timeout and signal broadcasting.
/// Uses atomic operations and async notifications for thread-safe coordination.
#[derive(Debug)]
pub struct ShutdownCoordinator {
    shutdown_signal: Arc<Notify>,
    is_shutting_down: Arc<AtomicBool>,
    timeout_duration: Duration,
}

impl ShutdownCoordinator {
    /// Minimum allowed shutdown timeout in seconds
    const MIN_TIMEOUT_SECONDS: u64 = 1;
    /// Maximum allowed shutdown timeout in seconds
    const MAX_TIMEOUT_SECONDS: u64 = 300;

    /// Create new shutdown coordinator with validated timeout
    ///
    /// Timeout must be between 1 and 300 seconds.
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

        if timeout_seconds > Self::MAX_TIMEOUT_SECONDS {
            return Err(LoadBalancerError::configuration(
                format!(
                    "Shutdown timeout must not exceed {} seconds, got {}",
                    Self::MAX_TIMEOUT_SECONDS,
                    timeout_seconds
                )
            ));
        }

        Ok(Self {
            shutdown_signal: Arc::new(Notify::new()),
            is_shutting_down: Arc::new(AtomicBool::new(false)),
            timeout_duration: Duration::from_secs(timeout_seconds),
        })
    }

    /// Trigger graceful shutdown
    ///
    /// Sets shutdown flag and notifies all waiting tasks.
    /// This method is idempotent.
    pub fn shutdown(&self) {
        self.is_shutting_down.store(true, Ordering::SeqCst);
        self.shutdown_signal.notify_waiters();
        tracing::info!("Shutdown initiated");
    }

    /// Check if shutdown has been triggered
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::SeqCst)
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
        let coordinator = ShutdownCoordinator::new(30).unwrap();
        assert!(!coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_validation_min() {
        let result = ShutdownCoordinator::new(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 1 seconds"));
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_validation_max() {
        let result = ShutdownCoordinator::new(301);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not exceed 300 seconds"));
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let coordinator = ShutdownCoordinator::new(30).unwrap();
        
        assert!(!coordinator.is_shutting_down());
        
        coordinator.shutdown();
        
        assert!(coordinator.is_shutting_down());
    }
}
