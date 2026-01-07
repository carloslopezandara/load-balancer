/// Worker metrics domain model - Pure entity with atomic fields
/// 
/// This is a pure domain entity that holds performance metrics for a worker.
/// All business logic (EMA calculations) is handled by the MetricsCollector service.

use std::sync::atomic::{AtomicU64, Ordering};
use crate::domain::LoadBalancerError;
use crate::utils::constants::metrics::FIXED_POINT_SCALE;

/// Performance metrics for an individual worker using EMA
/// 
/// Stores metrics using atomic types for thread-safe access.
/// EMA calculations are handled by MetricsCollector service.
#[derive(Debug)]
pub struct WorkerMetrics {
    /// Exponential moving average of response time in milliseconds
    ema_latency_ms: AtomicU64,
    /// Exponential moving average of error rate (stored as u64 * 10000 for precision)
    /// e.g., 0.05 (5%) stored as 500
    ema_error_rate: AtomicU64,
    /// Total sample count (for min_samples threshold validation)
    sample_count: AtomicU64,
    /// Alpha coefficient for EMA (stored as u32 * 10000)
    /// e.g., alpha=0.1 stored as 1000
    alpha: u32,
}

impl WorkerMetrics {
    /// Create new metrics with validated alpha coefficient
    /// 
    /// # Errors
    /// Returns error if alpha is not in range (0.0, 1.0]
    /// 
    /// # Examples
    /// ```
    /// use load_balancer::domain::WorkerMetrics;
    /// 
    /// let metrics = WorkerMetrics::new(0.1).unwrap();
    /// assert_eq!(metrics.get_sample_count(), 0);
    /// ```
    pub fn new(alpha: f64) -> Result<Self, LoadBalancerError> {
        if alpha <= 0.0 || alpha > 1.0 {
            return Err(LoadBalancerError::configuration(
                format!("Alpha must be in range (0.0, 1.0], got: {}", alpha)
            ));
        }
        
        Ok(Self {
            ema_latency_ms: AtomicU64::new(0),
            ema_error_rate: AtomicU64::new(0),
            sample_count: AtomicU64::new(0),
            alpha: (alpha * FIXED_POINT_SCALE as f64) as u32,
        })
    }
    
    pub fn get_sample_count(&self) -> u64 {
        self.sample_count.load(Ordering::Relaxed)
    }
    
    /// Get current EMA of latency in milliseconds (raw value)
    pub fn get_ema_latency_ms(&self) -> u64 {
        self.ema_latency_ms.load(Ordering::Relaxed)
    }
    
    /// Get current EMA of error rate (raw fixed-point value * 10000)
    pub fn get_ema_error_rate(&self) -> u64 {
        self.ema_error_rate.load(Ordering::Relaxed)
    }
    
    /// Get alpha coefficient (fixed-point value * 10000)
    pub fn get_alpha(&self) -> u32 {
        self.alpha
    }
    
    pub fn increment_sample_count(&self) {
        self.sample_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn set_ema_latency_ms(&self, value: u64) {
        self.ema_latency_ms.store(value, Ordering::Relaxed);
    }
    
    pub fn set_ema_error_rate(&self, value: u64) {
        self.ema_error_rate.store(value, Ordering::Relaxed);
    }
    
    /// Atomically update EMA latency if current value matches expected
    /// 
    /// Returns Ok(previous_value) on success, Err(current_value) on failure
    pub fn compare_exchange_ema_latency(&self, current: u64, new: u64) -> Result<u64, u64> {
        self.ema_latency_ms
            .compare_exchange(current, new, Ordering::Relaxed, Ordering::Relaxed)
    }
    
    /// Atomically update EMA error rate if current value matches expected
    /// 
    /// Returns Ok(previous_value) on success, Err(current_value) on failure
    pub fn compare_exchange_ema_error_rate(&self, current: u64, new: u64) -> Result<u64, u64> {
        self.ema_error_rate
            .compare_exchange(current, new, Ordering::Relaxed, Ordering::Relaxed)
    }
    
    pub fn request_count(&self) -> u64 {
        self.get_sample_count()
    }
    
    pub fn average_latency_ms(&self) -> u64 {
        self.get_ema_latency_ms()
    }
    
    pub fn error_rate(&self) -> f64 {
        self.get_ema_error_rate() as f64 / FIXED_POINT_SCALE as f64
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self::new(0.1).expect("Default alpha 0.1 is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_alpha_values() {
        assert!(WorkerMetrics::new(0.1).is_ok());
        assert!(WorkerMetrics::new(0.5).is_ok());
        assert!(WorkerMetrics::new(1.0).is_ok());
        assert!(WorkerMetrics::new(0.001).is_ok());
    }

    #[test]
    fn test_alpha_zero_fails() {
        let result = WorkerMetrics::new(0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Alpha must be in range"));
    }

    #[test]
    fn test_alpha_negative_fails() {
        let result = WorkerMetrics::new(-0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_alpha_greater_than_one_fails() {
        let result = WorkerMetrics::new(1.1);
        assert!(result.is_err());
    }

    // ========================================================================
    // Getter Tests
    // ========================================================================

    #[test]
    fn test_initial_state() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        assert_eq!(metrics.get_sample_count(), 0);
        assert_eq!(metrics.get_ema_latency_ms(), 0);
        assert_eq!(metrics.get_ema_error_rate(), 0);
        assert_eq!(metrics.get_alpha(), 1000); // 0.1 * 10000
    }

    #[test]
    fn test_request_count_alias() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        assert_eq!(metrics.request_count(), metrics.get_sample_count());
    }

    #[test]
    fn test_average_latency_alias() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        assert_eq!(metrics.average_latency_ms(), metrics.get_ema_latency_ms());
    }

    #[test]
    fn test_error_rate_conversion() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        metrics.set_ema_error_rate(2500); // 2500 / 10000 = 0.25
        assert_eq!(metrics.error_rate(), 0.25);
    }

    // ========================================================================
    // Setter Tests
    // ========================================================================

    #[test]
    fn test_increment_sample_count() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        assert_eq!(metrics.get_sample_count(), 0);
        metrics.increment_sample_count();
        assert_eq!(metrics.get_sample_count(), 1);
        metrics.increment_sample_count();
        assert_eq!(metrics.get_sample_count(), 2);
    }

    #[test]
    fn test_set_ema_latency() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        metrics.set_ema_latency_ms(150);
        assert_eq!(metrics.get_ema_latency_ms(), 150);
    }

    #[test]
    fn test_set_ema_error_rate() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        metrics.set_ema_error_rate(3000); // 30%
        assert_eq!(metrics.get_ema_error_rate(), 3000);
        assert_eq!(metrics.error_rate(), 0.3);
    }

    // ========================================================================
    // Compare-Exchange Tests
    // ========================================================================

    #[test]
    fn test_compare_exchange_latency_success() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        metrics.set_ema_latency_ms(100);
        let result = metrics.compare_exchange_ema_latency(100, 200);
        assert!(result.is_ok());
        assert_eq!(metrics.get_ema_latency_ms(), 200);
    }

    #[test]
    fn test_compare_exchange_latency_failure() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        metrics.set_ema_latency_ms(100);
        let result = metrics.compare_exchange_ema_latency(99, 200); // Wrong current
        assert!(result.is_err());
        assert_eq!(metrics.get_ema_latency_ms(), 100); // Unchanged
    }

    #[test]
    fn test_compare_exchange_error_rate_success() {
        let metrics = WorkerMetrics::new(0.1).unwrap();
        
        metrics.set_ema_error_rate(1000);
        let result = metrics.compare_exchange_ema_error_rate(1000, 2000);
        assert!(result.is_ok());
        assert_eq!(metrics.get_ema_error_rate(), 2000);
    }

    // ========================================================================
    // Concurrency Tests
    // ========================================================================

    #[test]
    fn test_concurrent_sample_count_increments() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(WorkerMetrics::new(0.1).unwrap());
        let mut handles = vec![];

        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.increment_sample_count();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.get_sample_count(), 1000);
    }

    #[test]
    fn test_default_uses_valid_alpha() {
        let metrics = WorkerMetrics::default();
        assert_eq!(metrics.get_alpha(), 1000); // 0.1 * 10000
        assert_eq!(metrics.get_sample_count(), 0);
    }
}
