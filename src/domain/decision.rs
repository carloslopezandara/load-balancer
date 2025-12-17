/// Decision engine domain types for adaptive load balancing

use std::time::Duration;
use crate::domain::strategy::StrategyType;
use crate::utils::constants::adaptive_defaults;

/// Thresholds for triggering strategy switches
#[derive(Debug, Clone)]
pub struct DecisionThresholds {
    /// Maximum acceptable average response time in milliseconds
    pub high_latency_ms: u64,
    /// Maximum acceptable error rate (0.0 to 1.0)
    pub high_error_rate: f64,
    /// Minimum number of requests before making decisions
    pub min_samples: u64,
    /// Minimum time between strategy switches
    pub cooldown: Duration,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            high_latency_ms: adaptive_defaults::HIGH_LATENCY_MS,
            high_error_rate: adaptive_defaults::HIGH_ERROR_RATE,
            min_samples: adaptive_defaults::MIN_SAMPLES,
            cooldown: Duration::from_secs(adaptive_defaults::COOLDOWN_SECONDS),
        }
    }
}

/// Reasons for triggering a strategy switch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchReason {
    HighLatency,
    HighErrorRate,
}

/// Decision result from evaluating metrics
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    KeepCurrent,
    SwitchTo {
        strategy: StrategyType,
        reason: SwitchReason,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_thresholds() {
        let thresholds = DecisionThresholds::default();
        assert_eq!(thresholds.high_latency_ms, adaptive_defaults::HIGH_LATENCY_MS);
        assert_eq!(thresholds.high_error_rate, adaptive_defaults::HIGH_ERROR_RATE);
        assert_eq!(thresholds.min_samples, adaptive_defaults::MIN_SAMPLES);
    }

    #[test]
    fn test_decision_equality() {
        let decision1 = Decision::KeepCurrent;
        let decision2 = Decision::KeepCurrent;
        assert_eq!(decision1, decision2);

        let decision3 = Decision::SwitchTo {
            strategy: StrategyType::LeastConnections,
            reason: SwitchReason::HighLatency,
        };
        let decision4 = Decision::SwitchTo {
            strategy: StrategyType::LeastConnections,
            reason: SwitchReason::HighLatency,
        };
        assert_eq!(decision3, decision4);
    }

    #[test]
    fn test_switch_reasons() {
        let reason1 = SwitchReason::HighLatency;
        let reason2 = SwitchReason::HighErrorRate;
        assert_ne!(reason1, reason2);
    }

    #[test]
    fn test_custom_thresholds() {
        let thresholds = DecisionThresholds {
            high_latency_ms: 1000,
            high_error_rate: 0.15,
            min_samples: 20,
            cooldown: Duration::from_secs(120),
        };
        assert_eq!(thresholds.high_latency_ms, 1000);
        assert_eq!(thresholds.high_error_rate, 0.15);
        assert_eq!(thresholds.min_samples, 20);
        assert_eq!(thresholds.cooldown, Duration::from_secs(120));
    }
}
