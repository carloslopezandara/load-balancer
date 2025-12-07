use std::sync::atomic::{AtomicUsize, Ordering};

/// Available load balancing strategies
#[derive(Debug)]
pub enum LoadBalancingStrategy {
    RoundRobin { current: AtomicUsize },
    LeastConnections { connections: Vec<AtomicUsize> },
}

impl LoadBalancingStrategy {
    /// Selects the next worker based on the current strategy
    /// 
    /// Returns the index of the selected worker from the worker pool
    pub fn select_worker(&self, worker_count: usize) -> usize {
        match self {
            LoadBalancingStrategy::RoundRobin { current } => {
                // Take the current index and increments it atomically
                let current_worker = current.fetch_add(1, Ordering::Relaxed);
                // Returns the index of the selected worker
                current_worker % worker_count
            }
            LoadBalancingStrategy::LeastConnections { connections } => {
                let mut least_index = 0;
                // Least connections start with the highest possible value
                let mut least_connections = usize::MAX; 
                // Iterate through the connections to find the worker with the least connections
                for (i, conn) in connections.iter().enumerate() {
                    let conn_count = conn.load(Ordering::Relaxed);
                    if conn_count < least_connections {
                        least_connections = conn_count;
                        least_index = i;
                    }
                }
                // Return the index of that worker 
                least_index
            }
        }
    }

    /// Notifies that a connection to the specified worker has started
    /// 
    /// Used by LeastConnections strategy to track active connections
    pub fn connection_started(&self, worker_index: usize) {
        if let LoadBalancingStrategy::LeastConnections { connections } = self {
            if let Some(conn) = connections.get(worker_index) {
                conn.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Notifies that a connection to the specified worker has ended
    /// 
    /// Used by LeastConnections strategy to track active connections
    pub fn connection_ended(&self, worker_index: usize) {
        if let LoadBalancingStrategy::LeastConnections { connections } = self {
            connections[worker_index].fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Creates a new round-robin strategy instance
    pub fn new_round_robin() -> Self {
        LoadBalancingStrategy::RoundRobin {
            current: AtomicUsize::new(0),
        }
    }

    /// Creates a new least-connections strategy instance
    /// 
    /// Initializes connection counters for the specified number of workers
    pub fn new_least_connections(worker_count: usize) -> Self {
        let connections = (0..worker_count)
            .map(|_| AtomicUsize::new(0))
            .collect();
        LoadBalancingStrategy::LeastConnections { connections }
    }
}