//! Shutdown coordinator for graceful server shutdown

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};

/// Coordinates graceful shutdown of the load balancer
pub struct ShutdownCoordinator {
    shutdown_signal: Arc<Notify>,
    is_shutting_down: Arc<AtomicBool>,
    timeout_duration: Duration,
}

impl ShutdownCoordinator {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            shutdown_signal: Arc::new(Notify::new()),
            is_shutting_down: Arc::new(AtomicBool::new(false)),
            timeout_duration: Duration::from_secs(timeout_seconds),
        }
    }

    /// Trigger shutdown
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
    pub async fn wait_for_shutdown(&self) {
        self.shutdown_signal.notified().await;
    }

    /// Wait for shutdown with timeout
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

    /// Get a cloneable shutdown signal
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
    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_shutdown_coordinator_creation() {
        let coordinator = ShutdownCoordinator::new(30);
        assert!(!coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let coordinator = ShutdownCoordinator::new(30);
        
        assert!(!coordinator.is_shutting_down());
        
        coordinator.shutdown();
        
        assert!(coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        let coordinator = Arc::new(ShutdownCoordinator::new(30));
        let coordinator_clone = coordinator.clone();

        let handle = tokio::spawn(async move {
            coordinator_clone.wait_for_shutdown().await;
        });

        sleep(Duration::from_millis(10)).await;
        coordinator.shutdown();

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_shutdown_signal_subscribe() {
        let coordinator = ShutdownCoordinator::new(30);
        let signal = coordinator.subscribe();

        let handle = tokio::spawn(async move {
            signal.wait().await;
        });

        sleep(Duration::from_millis(10)).await;
        coordinator.shutdown();

        handle.await.unwrap();
    }
}
