//! Decision Engine Service - adaptive load balancing logic
//!
//! This service evaluates worker metrics and decides when to switch
//! load balancing strategies based on performance thresholds and rate limiting.

use std::sync::{Arc, RwLock};
use std::time::Instant;
use crate::domain::{Decision, DecisionThresholds, LoadBalancerError, Result, StrategyType, SwitchReason};
use crate::services::MetricsCollector;

/// State of the current strategy and last switch time
#[derive(Clone)]
struct StrategyState {
    current: StrategyType,
    last_switch: Option<Instant>,
}

/// Decision engine for adaptive load balancing
///
/// Analyzes worker metrics and makes decisions about strategy switches
/// with rate limiting to prevent excessive changes.
pub struct DecisionEngine {
    thresholds: DecisionThresholds,
    /// Current strategy and last switch time protected by a single lock
    /// to prevent deadlocks from holding two separate locks
    state: RwLock<StrategyState>,
    metrics: Arc<MetricsCollector>,
}

impl DecisionEngine {
    /// Create new decision engine with default thresholds
    pub fn new(initial_strategy: StrategyType, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            thresholds: DecisionThresholds::default(),
            state: RwLock::new(StrategyState {
                current: initial_strategy,
                last_switch: None,
            }),
            metrics,
        }
    }

    /// Create new decision engine with custom thresholds
    pub fn with_config(
        initial_strategy: StrategyType,
        thresholds: DecisionThresholds,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            thresholds,
            state: RwLock::new(StrategyState {
                current: initial_strategy,
                last_switch: None,
            }),
            metrics,
        }
    }

    /// Get current strategy
    pub fn current_strategy(&self) -> Result<StrategyType> {
        self.state
            .read()
            .map(|guard| guard.current.clone())
            .map_err(|_| LoadBalancerError::concurrency("failed to read current strategy"))
    }

    /// Check if enough time has passed since last switch
    pub fn can_switch(&self) -> Result<bool> {
        let state = self.state
            .read()
            .map_err(|_| LoadBalancerError::concurrency("failed to check switch cooldown"))?;
        
        Ok(state.last_switch.map_or(true, |last| {
            last.elapsed() >= self.thresholds.cooldown
        }))
    }

    /// Evaluate metrics and decide whether to switch strategies
    pub fn evaluate(&self) -> Result<Decision> {
        let worker_count = self.metrics.all_metrics().len();
        
        if worker_count == 0 {
            return Ok(Decision::KeepCurrent);
        }

        let current = self.current_strategy()?;

        if !self.can_switch()? {
            tracing::debug!(
                current_strategy = %current.as_str(),
                cooldown_seconds = self.thresholds.cooldown.as_secs(),
                "Switch cooldown active"
            );
            return Ok(Decision::KeepCurrent);
        }

        // Calculate aggregate metrics across all workers
        let mut total_requests = 0u64;
        let mut high_latency_count = 0;
        let mut high_error_rate_count = 0;

        for i in 0..worker_count {
            if let Some(worker_metrics) = self.metrics.get_worker_metrics(i) {
                let request_count = worker_metrics.request_count();
                total_requests += request_count;

                // Only evaluate workers with sufficient samples
                if request_count >= self.thresholds.min_samples {
                    // Check latency using MetricsCollector calculations
                    if let Some(avg_latency) = self.metrics.average_response_time_ms(i) {
                        if avg_latency > self.thresholds.high_latency_ms {
                            high_latency_count += 1;
                        }
                    }

                    // Check error rate using MetricsCollector calculations
                    let error_rate = self.metrics.error_rate(i);
                    if error_rate > self.thresholds.high_error_rate {
                        high_error_rate_count += 1;
                    }
                }
            }
        }

        // Not enough data to make a decision
        if total_requests < self.thresholds.min_samples {
            tracing::debug!(
                total_requests = total_requests,
                min_samples = self.thresholds.min_samples,
                "Insufficient samples for decision"
            );
            return Ok(Decision::KeepCurrent);
        }

        // Decision logic: switch if majority of workers have issues
        let majority_threshold = (worker_count / 2) + 1;

        // High latency detected - switch to LeastConnections
        if high_latency_count >= majority_threshold && current != StrategyType::LeastConnections {
            tracing::info!(
                high_latency_count = high_latency_count,
                worker_count = worker_count,
                threshold_ms = self.thresholds.high_latency_ms,
                current_strategy = %current.as_str(),
                new_strategy = "least_connections",
                "High latency detected, switching strategy"
            );
            return Ok(Decision::SwitchTo {
                strategy: StrategyType::LeastConnections,
                reason: SwitchReason::HighLatency,
            });
        }

        // High error rate detected - switch to RoundRobin for fair distribution
        if high_error_rate_count >= majority_threshold && current != StrategyType::RoundRobin {
            tracing::info!(
                high_error_rate_count = high_error_rate_count,
                worker_count = worker_count,
                threshold_rate = self.thresholds.high_error_rate,
                current_strategy = %current.as_str(),
                new_strategy = "round_robin",
                "High error rate detected, switching strategy"
            );
            return Ok(Decision::SwitchTo {
                strategy: StrategyType::RoundRobin,
                reason: SwitchReason::HighErrorRate,
            });
        }

        Ok(Decision::KeepCurrent)
    }

    /// Apply a decision by updating current strategy and timestamp
    /// Uses a single lock to prevent deadlocks
    pub fn apply_decision(&self, decision: &Decision) -> Result<()> {
        if let Decision::SwitchTo { strategy, reason } = decision {
            let mut state = self.state
                .write()
                .map_err(|_| LoadBalancerError::concurrency("failed to update strategy"))?;

            state.current = strategy.clone();
            state.last_switch = Some(Instant::now());

            tracing::info!(
                new_strategy = %strategy.as_str(),
                reason = ?reason,
                "Strategy switch applied"
            );
        }
        Ok(())
    }

    /// Get the time of the last strategy switch
    pub fn last_switch_time(&self) -> Result<Option<Instant>> {
        self.state
            .read()
            .map(|guard| guard.last_switch)
            .map_err(|_| LoadBalancerError::concurrency("failed to read last switch time"))
    }

    /// Get remaining cooldown time in seconds
    pub fn cooldown_remaining_seconds(&self) -> Result<Option<u64>> {
        let last_switch = self.last_switch_time()?;
        
        Ok(match last_switch {
            None => None,
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= self.thresholds.cooldown {
                    None // Cooldown expired
                } else {
                    let remaining = self.thresholds.cooldown - elapsed;
                    Some(remaining.as_secs())
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_zero_workers_returns_keep_current() {
        let metrics = Arc::new(MetricsCollector::new(0));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics);
        
        let decision = engine.evaluate().unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_insufficient_samples_no_switch() {
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics.clone());
        
        // Only 2 samples, below threshold of 10
        metrics.record_success(0, Duration::from_millis(1000));
        metrics.record_success(1, Duration::from_millis(1000));
        
        let decision = engine.evaluate().unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_high_latency_majority_triggers_switch() {
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics.clone());
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate().unwrap();
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
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::new(StrategyType::LeastConnections, metrics.clone());
        
        for i in 0..2 {
            for _ in 0..8 {
                metrics.record_success(i, Duration::from_millis(100));
            }
            for _ in 0..3 {
                metrics.record_error(i, Duration::from_millis(100));
            }
        }
        
        let decision = engine.evaluate().unwrap();
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
        let metrics = Arc::new(MetricsCollector::new(3));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics.clone());
        
        for i in 0..3 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(100));
            }
        }
        
        // Only worker 0 has high latency (1 of 3, not majority)
        for _ in 0..10 {
            metrics.record_success(0, Duration::from_millis(600));
        }
        
        let decision = engine.evaluate().unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_already_using_optimal_strategy_no_switch() {
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::new(StrategyType::LeastConnections, metrics.clone());
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate().unwrap();
        assert_eq!(decision, Decision::KeepCurrent);
    }

    #[test]
    fn test_rate_limiting_enforced() {
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics.clone());
        
        for i in 0..2 {
            for _ in 0..15 {
                metrics.record_success(i, Duration::from_millis(600));
            }
        }
        
        let decision = engine.evaluate().unwrap();
        engine.apply_decision(&decision).unwrap();
        
        assert!(!engine.can_switch().unwrap());
        
        let decision2 = engine.evaluate().unwrap();
        assert_eq!(decision2, Decision::KeepCurrent);
    }

    #[test]
    fn test_single_worker_majority_logic() {
        let metrics = Arc::new(MetricsCollector::new(1));
        let engine = DecisionEngine::new(StrategyType::RoundRobin, metrics.clone());
        
        for _ in 0..15 {
            metrics.record_success(0, Duration::from_millis(600));
        }
        
        // Single worker with high latency should trigger switch (1 >= majority of 1)
        let decision = engine.evaluate().unwrap();
        assert!(matches!(decision, Decision::SwitchTo { .. }));
    }

    #[test]
    fn test_extreme_thresholds() {
        let thresholds = DecisionThresholds {
            high_latency_ms: 0,
            high_error_rate: 0.0,
            min_samples: 1,
            cooldown: Duration::from_secs(60),
        };
        let metrics = Arc::new(MetricsCollector::new(2));
        let engine = DecisionEngine::with_config(StrategyType::RoundRobin, thresholds, metrics.clone());
        
        metrics.record_success(0, Duration::from_millis(1));
        metrics.record_success(1, Duration::from_millis(1));
        
        // Even 1ms latency should trigger with threshold=0
        let decision = engine.evaluate().unwrap();
        assert!(matches!(decision, Decision::SwitchTo { .. }));
    }
}
