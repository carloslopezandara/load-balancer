//! Metrics Service - metrics collection and calculation
//!
//! This service handles metrics collection for all workers including
//! recording requests/errors and calculating statistics like error rates
//! and average response times.

use std::time::Duration;
use crate::domain::WorkerMetrics;

/// Metrics collection service for all workers
/// 
/// Provides validation, recording, and calculation of performance metrics
/// across all workers in the load balancer.
#[derive(Debug)]
pub struct MetricsCollector {
    workers: Vec<WorkerMetrics>,
}

impl MetricsCollector {
    /// Create new metrics collector for specified number of workers
    pub fn new(worker_count: usize) -> Self {
        let workers = (0..worker_count)
            .map(|_| WorkerMetrics::new())
            .collect();
            
        Self { workers }
    }
    
    /// Get metrics for a specific worker by index
    pub fn get_worker_metrics(&self, index: usize) -> Option<&WorkerMetrics> {
        self.workers.get(index)
    }
    
    /// Get metrics for all workers as a slice
    pub fn all_metrics(&self) -> &[WorkerMetrics] {
        &self.workers
    }
    
    /// Record a successful request for a worker
    pub fn record_success(&self, worker_index: usize, duration: Duration) {
        if let Some(metrics) = self.workers.get(worker_index) {
            metrics.record_success(duration);
        } else {
            tracing::warn!("Invalid worker_index for metrics: {}", worker_index);
        }
    }
    
    /// Record a failed request for a worker
    pub fn record_error(&self, worker_index: usize, duration: Duration) {
        if let Some(metrics) = self.workers.get(worker_index) {
            metrics.record_error(duration);
        } else {
            tracing::warn!("Invalid worker_index for metrics: {}", worker_index);
        }
    }
    
    /// Calculate error rate for a specific worker (0.0 to 1.0)
    pub fn error_rate(&self, worker_index: usize) -> f64 {
        if let Some(metrics) = self.workers.get(worker_index) {
            let requests = metrics.request_count();
            if requests == 0 {
                return 0.0;
            }
            
            let errors = metrics.error_count();
            errors as f64 / requests as f64
        } else {
            0.0
        }
    }
    
    /// Calculate average response time in milliseconds
    pub fn average_response_time_ms(&self, worker_index: usize) -> Option<u64> {
        if let Some(metrics) = self.workers.get(worker_index) {
            let requests = metrics.request_count();
            if requests == 0 {
                return None;
            }
            
            let total_ms = metrics.total_response_time_ms();
            Some(total_ms / requests)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_collector_edge_case() {
        let collector = MetricsCollector::new(0);
        
        assert_eq!(collector.all_metrics().len(), 0);
        assert!(collector.get_worker_metrics(0).is_none());
    }

    #[test]
    fn test_calculations_with_mixed_requests() {
        let collector = MetricsCollector::new(1);
        
        collector.record_success(0, Duration::from_millis(100));
        collector.record_success(0, Duration::from_millis(200));
        collector.record_error(0, Duration::from_millis(300));
        collector.record_success(0, Duration::from_millis(400));
        
        assert_eq!(collector.error_rate(0), 0.25);
        assert_eq!(collector.average_response_time_ms(0), Some(250));
    }

    #[test]
    fn test_multiple_workers_independent() {
        let collector = MetricsCollector::new(3);
        
        collector.record_success(0, Duration::from_millis(100));
        collector.record_error(1, Duration::from_millis(200));
        collector.record_success(2, Duration::from_millis(150));
        
        assert_eq!(collector.error_rate(0), 0.0);
        assert_eq!(collector.error_rate(1), 1.0);
        assert_eq!(collector.error_rate(2), 0.0);
    }

    #[test]
    fn test_invalid_worker_index_handling() {
        let collector = MetricsCollector::new(2);
        
        assert!(collector.get_worker_metrics(2).is_none());
        assert_eq!(collector.error_rate(100), 0.0);
        assert_eq!(collector.average_response_time_ms(100), None);
        
        // Should not panic, just log warning
        collector.record_success(5, Duration::from_millis(100));
        collector.record_error(10, Duration::from_millis(100));
    }

    #[test]
    fn test_zero_duration_edge_case() {
        let collector = MetricsCollector::new(1);
        
        collector.record_success(0, Duration::ZERO);
        collector.record_success(0, Duration::from_millis(100));
        
        assert_eq!(collector.average_response_time_ms(0), Some(50));
    }

    #[test]
    fn test_concurrent_access_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(MetricsCollector::new(3));
        let mut handles = vec![];

        for worker_idx in 0..3 {
            let collector_clone = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    if i % 10 == 0 {
                        collector_clone.record_error(worker_idx, Duration::from_millis(10));
                    } else {
                        collector_clone.record_success(worker_idx, Duration::from_millis(10));
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        for i in 0..3 {
            assert_eq!(collector.get_worker_metrics(i).unwrap().request_count(), 100);
            assert_eq!(collector.error_rate(i), 0.1);
        }
    }
}
