//! Decision Engine Service - adaptive load balancing logic
//!
//! This service evaluates worker metrics and decides when to switch
//! load balancing strategies based on performance thresholds and rate limiting.

use std::sync::RwLock;
use std::time::{Duration, Instant};
use crate::domain::{Decision, DecisionThresholds, LoadBalancerError, Result, StrategyType, SwitchReason};
use crate::services::MetricsCollector;

/// Decision engine for adaptive load balancing
///
/// Analyzes worker metrics and makes decisions about strategy switches
/// with rate limiting to prevent excessive changes.
pub struct DecisionEngine {
    thresholds: DecisionThresholds,
    current_strategy: RwLock<StrategyType>,
    last_switch: RwLock<Option<Instant>>,
    switch_cooldown: Duration,
}

impl DecisionEngine {
    /// Create new decision engine with default thresholds
    pub fn new(initial_strategy: StrategyType) -> Self {
        Self {
            thresholds: DecisionThresholds::default(),
            current_strategy: RwLock::new(initial_strategy),
            last_switch: RwLock::new(None),
            switch_cooldown: Duration::from_secs(60),
        }
    }

    /// Create new decision engine with custom thresholds
    pub fn with_thresholds(initial_strategy: StrategyType, thresholds: DecisionThresholds) -> Self {
        Self {
            thresholds,
            current_strategy: RwLock::new(initial_strategy),
            last_switch: RwLock::new(None),
            switch_cooldown: Duration::from_secs(60),
        }
    }

    /// Get current strategy
    pub fn current_strategy(&self) -> Result<StrategyType> {
        self.current_strategy
            .read()
            .map(|guard| guard.clone())
            .map_err(|_| LoadBalancerError::concurrency("failed to read current strategy"))
    }

    /// Check if enough time has passed since last switch
    pub fn can_switch(&self) -> Result<bool> {
        let last_switch = self.last_switch
            .read()
            .map_err(|_| LoadBalancerError::concurrency("failed to check switch cooldown"))?;
        
        Ok(match *last_switch {
            None => true,
            Some(last) => last.elapsed() >= self.switch_cooldown,
        })
    }

    /// Evaluate metrics and decide whether to switch strategies
    pub fn evaluate(&self, metrics: &MetricsCollector) -> Result<Decision> {
        let worker_count = metrics.all_metrics().len();
        
        if worker_count == 0 {
            return Ok(Decision::KeepCurrent);
        }

        if !self.can_switch()? {
            tracing::debug!("Switch cooldown active, keeping current strategy");
            return Ok(Decision::KeepCurrent);
        }

        let current = self.current_strategy()?;

        // Calculate aggregate metrics across all workers
        let mut total_requests = 0u64;
        let mut high_latency_count = 0;
        let mut high_error_rate_count = 0;

        for i in 0..worker_count {
            if let Some(worker_metrics) = metrics.get_worker_metrics(i) {
                let request_count = worker_metrics.request_count();
                total_requests += request_count;

                // Only evaluate workers with sufficient samples
                if request_count >= self.thresholds.min_samples {
                    // Check latency using MetricsCollector calculations
                    if let Some(avg_latency) = metrics.average_response_time_ms(i) {
                        if avg_latency > self.thresholds.high_latency_ms {
                            high_latency_count += 1;
                        }
                    }

                    // Check error rate using MetricsCollector calculations
                    let error_rate = metrics.error_rate(i);
                    if error_rate > self.thresholds.high_error_rate {
                        high_error_rate_count += 1;
                    }
                }
            }
        }

        // Not enough data to make a decision
        if total_requests < self.thresholds.min_samples {
            tracing::debug!("Insufficient samples ({}) for decision", total_requests);
            return Ok(Decision::KeepCurrent);
        }

        // Decision logic: switch if majority of workers have issues
        let majority_threshold = (worker_count / 2) + 1;

        // High latency detected - switch to LeastConnections
        if high_latency_count >= majority_threshold && current != StrategyType::LeastConnections {
            tracing::info!(
                "High latency detected on {} workers, recommending switch to LeastConnections",
                high_latency_count
            );
            return Ok(Decision::SwitchTo {
                strategy: StrategyType::LeastConnections,
                reason: SwitchReason::HighLatency,
            });
        }

        // High error rate detected - switch to RoundRobin for fair distribution
        if high_error_rate_count >= majority_threshold && current != StrategyType::RoundRobin {
            tracing::info!(
                "High error rate detected on {} workers, recommending switch to RoundRobin",
                high_error_rate_count
            );
            return Ok(Decision::SwitchTo {
                strategy: StrategyType::RoundRobin,
                reason: SwitchReason::HighErrorRate,
            });
        }

        Ok(Decision::KeepCurrent)
    }

    /// Apply a decision by updating current strategy and timestamp
    pub fn apply_decision(&self, decision: &Decision) -> Result<()> {
        if let Decision::SwitchTo { strategy, reason } = decision {
            let mut current = self.current_strategy
                .write()
                .map_err(|_| LoadBalancerError::concurrency("failed to update strategy"))?;
            let mut last_switch = self.last_switch
                .write()
                .map_err(|_| LoadBalancerError::concurrency("failed to record switch timestamp"))?;

            *current = strategy.clone();
            *last_switch = Some(Instant::now());

            tracing::info!(
                "Strategy switched to {} due to {:?}",
                strategy.as_str(),
                reason
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_workers_returns_keep_current() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(0);
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_insufficient_samples_no_switch() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(2);
        
        // Only 2 samples, below threshold of 10
        metrics.record_success(0, Duration::from_millis(1000));
        metrics.record_success(1, Duration::from_millis(1000));
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_high_latency_majority_triggers_switch() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(2);
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(
            decision,
            Decision::SwitchTo {
                strategy: StrategyType::LeastConnections,
                reason: SwitchReason::HighLatency,
            }
        );
    }

    #[test]
    fn test_high_error_rate_majority_triggers_switch() {
        let engine = DecisionEngine::new(StrategyType::LeastConnections);
        let metrics = MetricsCollector::new(2);
        
        for i in 0..2 {
            for _ in 0..8 {
                metrics.record_success(i, Duration::from_millis(100));
            }
            for _ in 0..3 {
                metrics.record_error(i, Duration::from_millis(100));
            }
        }
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(
            decision,
            Decision::SwitchTo {
                strategy: StrategyType::RoundRobin,
                reason: SwitchReason::HighErrorRate,
            }
        );
    }

    #[test]
    fn test_minority_issues_no_switch() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(3);
        
        for i in 0..3 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(100));
            }
        }
        
        // Only worker 0 has high latency (1 of 3, not majority)
        for _ in 0..10 {
            metrics.record_success(0, Duration::from_millis(600));
        }
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_already_using_optimal_strategy_no_switch() {
        let engine = DecisionEngine::new(StrategyType::LeastConnections);
        let metrics = MetricsCollector::new(2);
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate(&metrics).unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_rate_limiting_enforced() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(2);
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate(&metrics).unwrap();
        engine.apply_decision(&decision).unwrap();
        
        assert!(!engine.can_switch().unwrap());
        
        let decision2 = engine.evaluate(&metrics).unwrap();
        assert_eq!(decision2, Decision::KeepCurrent);
    }

    #[test]
    fn test_single_worker_majority_logic() {
        let engine = DecisionEngine::new(StrategyType::RoundRobin);
        let metrics = MetricsCollector::new(1);
        
        for _ in 0..15 {
            metrics.record_success(0, Duration::from_millis(600));
        }
        
        // Single worker with high latency should trigger switch (1 >= majority of 1)
        let decision = engine.evaluate(&metrics).unwrap();
        assert!(matches!(decision, Decision::SwitchTo { .. }));
    }

    #[test]
    fn test_extreme_thresholds() {
        let thresholds = DecisionThresholds {
            high_latency_ms: 0,
            high_error_rate: 0.0,
            min_samples: 1,
        };
        let engine = DecisionEngine::with_thresholds(StrategyType::RoundRobin, thresholds);
        let metrics = MetricsCollector::new(2);
        
        metrics.record_success(0, Duration::from_millis(1));
        metrics.record_success(1, Duration::from_millis(1));
        
        // Even 1ms latency should trigger with threshold=0
        let decision = engine.evaluate(&metrics).unwrap();
        assert!(matches!(decision, Decision::SwitchTo { .. }));
    }
}
