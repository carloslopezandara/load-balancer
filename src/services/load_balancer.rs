//! Load Balancer Service - core balancing logic
//!
//! This service handles the core load balancing functionality including
//! worker selection, strategy management, and connection tracking.
//! It's separated from HTTP concerns and request forwarding.

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::domain::{LoadBalancerError, Result, WorkerUrl};
use crate::load_balancing_strategy::LoadBalancingStrategy;

/// Load Balancer Service for handling core balancing logic
/// 
/// This service manages worker selection and strategy operations
/// without dealing with HTTP request forwarding.
pub struct LoadBalancerService {
    /// List of backend worker URLs (validated)
    worker_hosts: Vec<WorkerUrl>,
    /// Current load balancing strategy (protected by RwLock for thread safety)
    strategy: Arc<RwLock<LoadBalancingStrategy>>,
}

impl LoadBalancerService {
    /// Strategy name constants to avoid heap allocations
    const ROUND_ROBIN_NAME: &'static str = "round_robin";
    const LEAST_CONNECTIONS_NAME: &'static str = "least_connections";

    /// Create new load balancer service
    pub fn new(worker_hosts: Vec<WorkerUrl>, strategy: LoadBalancingStrategy) -> Result<Self> {
        if worker_hosts.is_empty() {
            return Err(LoadBalancerError::configuration("No worker hosts provided"));
        }

        Ok(LoadBalancerService {
            worker_hosts,
            strategy: Arc::new(RwLock::new(strategy)),
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
}
