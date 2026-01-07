//! Load Balancer Service - core balancing logic
//!
//! This service handles the core load balancing functionality including
//! worker selection, strategy management, connection tracking, and metrics collection.
//! It's separated from HTTP concerns and request forwarding.

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::domain::{Decision, LoadBalancerError, Result, StrategyType, WorkerUrl};
use crate::load_balancing_strategy::LoadBalancingStrategy;
use crate::services::{DecisionEngine, MetricsCollector};

/// Load Balancer Service for handling core balancing logic
/// 
/// This service manages worker selection, strategy operations, and metrics collection
/// without dealing with HTTP request forwarding.
pub struct LoadBalancerService {
    /// List of backend worker URLs (validated)
    worker_hosts: Vec<WorkerUrl>,
    /// Current load balancing strategy (protected by RwLock for thread safety)
    strategy: Arc<RwLock<LoadBalancingStrategy>>,
    /// Metrics collector for tracking worker performance
    metrics_collector: Arc<MetricsCollector>,
    /// Optional decision engine for adaptive load balancing
    decision_engine: Option<Arc<DecisionEngine>>,
}

impl LoadBalancerService {
    /// Strategy name constants to avoid heap allocations
    const ROUND_ROBIN_NAME: &'static str = "round_robin";
    const LEAST_CONNECTIONS_NAME: &'static str = "least_connections";

    /// Create new load balancer service without adaptive behavior
    pub fn new(worker_hosts: Vec<WorkerUrl>, strategy: LoadBalancingStrategy) -> Result<Self> {
        Self::with_adaptive(worker_hosts, strategy, false)
    }

    /// Create new load balancer service with optional adaptive behavior
    pub fn with_adaptive(
        worker_hosts: Vec<WorkerUrl>,
        strategy: LoadBalancingStrategy,
        is_adaptive: bool,
    ) -> Result<Self> {
        if worker_hosts.is_empty() {
            return Err(LoadBalancerError::configuration("No worker hosts provided"));
        }

        // MetricsCollector::new() now returns Result - propagate errors
        let metrics_collector = Arc::new(MetricsCollector::new(worker_hosts.len())?);
        
        let decision_engine = if is_adaptive {
            let initial_strategy = match strategy {
                LoadBalancingStrategy::RoundRobin { .. } => StrategyType::RoundRobin,
                LoadBalancingStrategy::LeastConnections { .. } => StrategyType::LeastConnections,
            };
            Some(Arc::new(DecisionEngine::new(initial_strategy, metrics_collector.clone())))
        } else {
            None
        };

        Ok(LoadBalancerService {
            worker_hosts,
            strategy: Arc::new(RwLock::new(strategy)),
            metrics_collector,
            decision_engine,
        })
    }

    /// Create new load balancer service with custom adaptive configuration
    pub fn with_adaptive_config(
        worker_hosts: Vec<WorkerUrl>,
        strategy: LoadBalancingStrategy,
        is_adaptive: bool,
        thresholds: crate::domain::DecisionThresholds,
        ema_alpha: f64,
    ) -> Result<Self> {
        if worker_hosts.is_empty() {
            return Err(LoadBalancerError::configuration("No worker hosts provided"));
        }

        // MetricsCollector with custom alpha from config - propagate validation errors
        let metrics_collector = Arc::new(MetricsCollector::with_alpha(worker_hosts.len(), ema_alpha)?);
        
        let decision_engine = if is_adaptive {
            let initial_strategy = match strategy {
                LoadBalancingStrategy::RoundRobin { .. } => StrategyType::RoundRobin,
                LoadBalancingStrategy::LeastConnections { .. } => StrategyType::LeastConnections,
            };
            Some(Arc::new(DecisionEngine::with_config(
                initial_strategy,
                thresholds,
                metrics_collector.clone(),
            )))
        } else {
            None
        };

        Ok(LoadBalancerService {
            worker_hosts,
            strategy: Arc::new(RwLock::new(strategy)),
            metrics_collector,
            decision_engine,
        })
    }

    /// Select the next worker based on the current load balancing strategy
    pub async fn select_worker(&self) -> Result<(usize, &WorkerUrl)> {
        if self.worker_hosts.is_empty() {
            return Err(LoadBalancerError::NoWorkersAvailable);
        }
        
        let strategy = self.strategy.read().await;
        let index = strategy.select_worker(self.worker_hosts.len())?;
        Ok((index, &self.worker_hosts[index]))
    }

    /// Notify that a connection has started for connection tracking
    pub async fn connection_started(&self, worker_index: usize) {
        let strategy = self.strategy.read().await;
        strategy.connection_started(worker_index);
    }

    /// Notify that a connection has ended for connection tracking
    pub async fn connection_ended(&self, worker_index: usize) {
        let strategy = self.strategy.read().await;
        strategy.connection_ended(worker_index);
    }

    /// Get the current strategy name
    pub async fn get_strategy_name(&self) -> &'static str {
        let strategy = self.strategy.read().await;
        match *strategy {
            LoadBalancingStrategy::RoundRobin { .. } => Self::ROUND_ROBIN_NAME,
            LoadBalancingStrategy::LeastConnections { .. } => Self::LEAST_CONNECTIONS_NAME,
        }
    }

    /// Change the load balancing strategy at runtime
    pub async fn set_strategy(&self, new_strategy: LoadBalancingStrategy) {
        let mut strategy = self.strategy.write().await;
        *strategy = new_strategy;
    }

    /// Get the current worker host list
    pub fn worker_hosts(&self) -> &Vec<WorkerUrl> {
        &self.worker_hosts
    }

    /// Get reference to the metrics collector
    pub fn metrics_collector(&self) -> &Arc<MetricsCollector> {
        &self.metrics_collector
    }

    /// Get reference to the decision engine if adaptive mode is enabled
    pub fn decision_engine(&self) -> Option<&Arc<DecisionEngine>> {
        self.decision_engine.as_ref()
    }

    /// Evaluate metrics and adapt strategy if needed (only if adaptive mode is enabled)
    pub async fn evaluate_and_adapt(&self) -> Result<()> {
        let Some(engine) = &self.decision_engine else {
            return Ok(());
        };

        let decision = engine.evaluate()?;

        if let Decision::SwitchTo { strategy, reason } = &decision {
            let new_strategy = match strategy {
                StrategyType::RoundRobin => LoadBalancingStrategy::new_round_robin(),
                StrategyType::LeastConnections => {
                    LoadBalancingStrategy::new_least_connections(self.worker_hosts.len())?
                }
            };

            self.set_strategy(new_strategy).await;
            engine.apply_decision(&decision)?;

            tracing::info!(
                new_strategy = %strategy.as_str(),
                reason = ?reason,
                worker_count = self.worker_hosts.len(),
                "Adaptive load balancing strategy switched"
            );
        }

        Ok(())
    }

    /// Check if adaptive mode is enabled
    pub fn is_adaptive(&self) -> bool {
        self.decision_engine.is_some()
    }
}
