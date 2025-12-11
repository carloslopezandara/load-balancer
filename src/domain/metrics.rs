/// Worker metrics domain model with atomic counters

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Performance metrics for an individual worker
/// 
/// Uses atomic operations for lock-free, thread-safe updates.
/// All metrics are cumulative from initialization.
#[derive(Debug)]
pub struct WorkerMetrics {
    request_count: AtomicU64,
    error_count: AtomicU64,
    total_response_time_ms: AtomicU64,
}

impl WorkerMetrics {
    /// Create new metrics with all counters at zero
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
        }
    }
    
    /// Record a successful request with its duration
    pub fn record_success(&self, duration: Duration) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms.fetch_add(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );
    }
    
    /// Record a failed request with its duration
    pub fn record_error(&self, duration: Duration) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.error_count.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms.fetch_add(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );
    }
    
    /// Get current request count
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }
    
    /// Get current error count
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }
    
    /// Get total response time in milliseconds
    pub fn total_response_time_ms(&self) -> u64 {
        self.total_response_time_ms.load(Ordering::Relaxed)
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let metrics = WorkerMetrics::new();
        
        assert_eq!(metrics.request_count(), 0);
        assert_eq!(metrics.error_count(), 0);
        assert_eq!(metrics.total_response_time_ms(), 0);
    }

    #[test]
    fn test_mixed_success_and_errors() {
        let metrics = WorkerMetrics::new();
        
        metrics.record_success(Duration::from_millis(100));
        metrics.record_error(Duration::from_millis(200));
        metrics.record_success(Duration::from_millis(300));
        
        assert_eq!(metrics.request_count(), 3);
        assert_eq!(metrics.error_count(), 1);
        assert_eq!(metrics.total_response_time_ms(), 600);
    }

    #[test]
    fn test_zero_duration_edge_case() {
        let metrics = WorkerMetrics::new();
        
        metrics.record_success(Duration::ZERO);
        metrics.record_success(Duration::from_millis(100));
        
        assert_eq!(metrics.request_count(), 2);
        assert_eq!(metrics.total_response_time_ms(), 100);
    }

    #[test]
    fn test_concurrent_updates() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(WorkerMetrics::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    if i % 10 == 0 {
                        metrics_clone.record_error(Duration::from_millis(10));
                    } else {
                        metrics_clone.record_success(Duration::from_millis(10));
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.request_count(), 1000);
        assert_eq!(metrics.error_count(), 100);
        assert_eq!(metrics.total_response_time_ms(), 10000);
    }
}
