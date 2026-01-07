//! Metrics Service - EMA-based metrics collection
//!
//! Handles metrics collection using Exponential Moving Average for adaptive decisions.
//! With default alpha=0.1, the system has a half-life of ~6.6 samples.

use std::time::Duration;
use crate::domain::{WorkerMetrics, LoadBalancerError};
use crate::utils::constants::metrics::FIXED_POINT_SCALE;

/// Metrics collection service for all workers
/// 
/// Manages EMA calculations and coordinates updates across workers.
/// Domain entities (WorkerMetrics) store data, this service handles business logic.
#[derive(Debug)]
pub struct MetricsCollector {
    workers: Vec<WorkerMetrics>,
}

impl MetricsCollector {
    /// Create new metrics collector with default alpha (0.1)
    /// 
    /// # Errors
    /// Returns error if worker_count is 0 or alpha is invalid
    pub fn new(worker_count: usize) -> Result<Self, LoadBalancerError> {
        Self::with_alpha(worker_count, 0.1)
    }
    
    /// Create new metrics collector with custom alpha coefficient
    /// 
    /// # Errors
    /// Returns error if worker_count is 0 or alpha is invalid
    pub fn with_alpha(worker_count: usize, alpha: f64) -> Result<Self, LoadBalancerError> {
        if worker_count == 0 {
            return Err(LoadBalancerError::configuration(
                "Worker count must be greater than 0"
            ));
        }
        
        let workers: Result<Vec<_>, _> = (0..worker_count)
            .map(|_| WorkerMetrics::new(alpha))
            .collect();
            
        Ok(Self { workers: workers? })
    }
    
    /// Get metrics for a specific worker by index
    pub fn get_worker_metrics(&self, index: usize) -> Option<&WorkerMetrics> {
        self.workers.get(index)
    }
    
    /// Get metrics for all workers as a slice
    pub fn all_metrics(&self) -> &[WorkerMetrics] {
        &self.workers
    }
    
    /// Update EMA for latency using lock-free compare-and-swap
    fn update_ema_latency(&self, worker_index: usize, sample_ms: u64) {
        if let Some(metrics) = self.workers.get(worker_index) {
            let alpha = metrics.get_alpha() as u64;
            
            loop {
                let old_ema = metrics.get_ema_latency_ms();
                
                // EMA formula: new = α * sample + (1-α) * old
                // Using fixed-point arithmetic: divide by FIXED_POINT_SCALE at the end
                let scale = FIXED_POINT_SCALE as u64;
                let new_ema = (alpha * sample_ms + (scale - alpha) * old_ema) / scale;
                
                if metrics.compare_exchange_ema_latency(old_ema, new_ema).is_ok() {
                    break;
                }
            }
        }
    }
    
    /// Update EMA for error rate using lock-free compare-and-swap
    fn update_ema_error_rate(&self, worker_index: usize, error_value: u64) {
        if let Some(metrics) = self.workers.get(worker_index) {
            let alpha = metrics.get_alpha() as u64;
            
            loop {
                let old_ema = metrics.get_ema_error_rate();
                
                // EMA formula: new = α * sample + (1-α) * old
                let scale = FIXED_POINT_SCALE as u64;
                let new_ema = (alpha * error_value + (scale - alpha) * old_ema) / scale;
                
                if metrics.compare_exchange_ema_error_rate(old_ema, new_ema).is_ok() {
                    break;
                }
            }
        }
    }
    
    pub fn record_success(&self, worker_index: usize, duration: Duration) {
        if let Some(metrics) = self.workers.get(worker_index) {
            metrics.increment_sample_count();
            self.update_ema_latency(worker_index, duration.as_millis() as u64);
            self.update_ema_error_rate(worker_index, 0); // 0 = no error
        } else {
            tracing::warn!("Invalid worker_index for metrics: {}", worker_index);
        }
    }
    
    pub fn record_error(&self, worker_index: usize, duration: Duration) {
        if let Some(metrics) = self.workers.get(worker_index) {
            metrics.increment_sample_count();
            self.update_ema_latency(worker_index, duration.as_millis() as u64);
            self.update_ema_error_rate(worker_index, FIXED_POINT_SCALE as u64); // FIXED_POINT_SCALE = 100% error
        } else {
            tracing::warn!("Invalid worker_index for metrics: {}", worker_index);
        }
    }
    
    pub fn error_rate(&self, worker_index: usize) -> f64 {
        self.workers
            .get(worker_index)
            .map(|m| m.error_rate())
            .unwrap_or(0.0)
    }
    
    pub fn average_response_time_ms(&self, worker_index: usize) -> Option<u64> {
        self.workers
            .get(worker_index)
            .map(|m| {
                let latency = m.average_latency_ms();
                if m.request_count() == 0 {
                    None
                } else {
                    Some(latency)
                }
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_workers_fails() {
        let result = MetricsCollector::new(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Worker count must be greater than 0"));
    }

    #[test]
    fn test_invalid_alpha_fails() {
        let result = MetricsCollector::with_alpha(2, 0.0);
        assert!(result.is_err());
        
        let result2 = MetricsCollector::with_alpha(2, -0.1);
        assert!(result2.is_err());
        
        let result3 = MetricsCollector::with_alpha(2, 1.5);
        assert!(result3.is_err());
    }

    #[test]
    fn test_valid_creation() {
        let collector = MetricsCollector::new(3).unwrap();
        assert_eq!(collector.all_metrics().len(), 3);
        
        let collector2 = MetricsCollector::with_alpha(2, 0.5).unwrap();
        assert_eq!(collector2.all_metrics().len(), 2);
    }

    #[test]
    fn test_ema_latency_updates() {
        let collector = MetricsCollector::with_alpha(1, 0.5).unwrap(); // 50% alpha for clearer math
        
        // First sample: EMA = 0.5 * 100 + 0.5 * 0 = 50ms
        collector.record_success(0, Duration::from_millis(100));
        assert_eq!(collector.get_worker_metrics(0).unwrap().request_count(), 1);
        assert_eq!(collector.average_response_time_ms(0).unwrap(), 50);
        
        // Second sample: EMA = 0.5 * 200 + 0.5 * 50 = 125ms
        collector.record_success(0, Duration::from_millis(200));
        assert_eq!(collector.get_worker_metrics(0).unwrap().request_count(), 2);
        assert_eq!(collector.average_response_time_ms(0).unwrap(), 125);
    }

    #[test]
    fn test_ema_error_rate_updates() {
        let collector = MetricsCollector::with_alpha(1, 0.5).unwrap();
        
        // First error: EMA = 0.5 * 1.0 + 0.5 * 0.0 = 0.5
        collector.record_error(0, Duration::from_millis(100));
        assert_eq!(collector.get_worker_metrics(0).unwrap().request_count(), 1);
        assert_eq!(collector.error_rate(0), 0.5);
        
        // Success: EMA = 0.5 * 0.0 + 0.5 * 0.5 = 0.25
        collector.record_success(0, Duration::from_millis(100));
        assert_eq!(collector.get_worker_metrics(0).unwrap().request_count(), 2);
        assert_eq!(collector.error_rate(0), 0.25);
    }

    #[test]
    fn test_calculations_with_mixed_requests() {
        let collector = MetricsCollector::new(1).unwrap();
        
        collector.record_success(0, Duration::from_millis(100));
        collector.record_success(0, Duration::from_millis(200));
        collector.record_error(0, Duration::from_millis(300));
        collector.record_success(0, Duration::from_millis(400));
        
        assert_eq!(collector.get_worker_metrics(0).unwrap().request_count(), 4);
        
        // With EMA (alpha=0.1), error rate is approximate
        let error_rate = collector.error_rate(0);
        assert!(error_rate > 0.0 && error_rate < 1.0, "Expected error rate between 0 and 1, got {}", error_rate);
        
        // EMA latency will be influenced by all samples but not exact average
        let avg_latency = collector.average_response_time_ms(0).unwrap();
        assert!(avg_latency > 0 && avg_latency < 400, "Expected latency between 0 and 400ms, got {}", avg_latency);
    }

    // ========================================================================
    // Multi-Worker Tests
    // ========================================================================

    #[test]
    fn test_multiple_workers_independent() {
        let collector = MetricsCollector::new(3).unwrap();
        
        collector.record_success(0, Duration::from_millis(100));
        collector.record_error(1, Duration::from_millis(200));
        collector.record_success(2, Duration::from_millis(150));
        
        // After single samples with alpha=0.1:
        // - Success: EMA error rate = 0.1 * 0.0 + 0.9 * 0.0 = 0.0
        // - Error: EMA error rate = 0.1 * 1.0 + 0.9 * 0.0 = 0.1
        assert!(collector.error_rate(0) < 0.05, "Worker 0 should have very low error rate");
        assert!(collector.error_rate(1) > 0.05, "Worker 1 should have elevated error rate");
        assert!(collector.error_rate(2) < 0.05, "Worker 2 should have very low error rate");
    }

    #[test]
    fn test_invalid_worker_index_handling() {
        let collector = MetricsCollector::new(2).unwrap();
        
        assert!(collector.get_worker_metrics(2).is_none());
        assert_eq!(collector.error_rate(100), 0.0);
        assert_eq!(collector.average_response_time_ms(100), None);
        
        // Should not panic, just log warning
        collector.record_success(5, Duration::from_millis(100));
        collector.record_error(10, Duration::from_millis(100));
    }

    #[test]
    fn test_zero_duration_edge_case() {
        let collector = MetricsCollector::new(1).unwrap();
        
        collector.record_success(0, Duration::ZERO);
        collector.record_success(0, Duration::from_millis(100));
        
        // With EMA, result is approximate (not exact average of 50)
        let avg = collector.average_response_time_ms(0).unwrap();
        assert!(avg < 100, "Average should be less than max sample (100ms), got {}", avg);
    }

    #[test]
    fn test_concurrent_access_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(MetricsCollector::new(3).unwrap());
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
            
            // With EMA, error rate converges towards 0.1 but may not be exact
            let error_rate = collector.error_rate(i);
            assert!(error_rate > 0.05 && error_rate < 0.15, 
                    "Expected error rate near 0.1, got {} for worker {}", error_rate, i);
        }
    }
}
