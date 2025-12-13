use load_balancer::load_balancing_strategy::least_connections::LeastConnections;
use load_balancer::load_balancing_strategy::traits::Strategy;
use std::sync::Arc;
use std::thread;
use std::collections::HashMap;

/// Test concurrent access maintains correct connection counts and distribution
/// 
/// This integration test verifies thread-safety and that the algorithm correctly 
/// balances load when multiple threads are starting/ending connections simultaneously.
/// Goes beyond basic unit tests to verify statistical distribution under heavy load.
#[test]
fn test_concurrent_access_with_distribution() {
    let lc = Arc::new(LeastConnections::new(3).unwrap());
    let iterations = 300;
    let mut handles = vec![];
    
    // Shared counter to track selections
    let results = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Test concurrent access with multiple threads
    for _ in 0..10 {
        let lc_clone = Arc::clone(&lc);
        let results_clone = Arc::clone(&results);
        let handle = thread::spawn(move || {
            let mut local_results = Vec::new();
            for i in 0..iterations {
                // Select worker
                let worker = lc_clone.select_worker(3).unwrap();
                local_results.push(worker);
                
                // Simulate connection lifecycle
                lc_clone.connection_started(worker);
                
                // Half the time, immediately end the connection
                if i % 2 == 0 {
                    lc_clone.connection_ended(worker);
                }
            }
            results_clone.lock().unwrap().extend(local_results);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify distribution is reasonably balanced (within 15% tolerance for concurrent access)
    let selections = results.lock().unwrap();
    let mut distribution: HashMap<usize, usize> = HashMap::new();
    for &worker in selections.iter() {
        *distribution.entry(worker).or_insert(0) += 1;
    }
    
    let total = selections.len();
    let expected_per_worker = total / 3;
    let tolerance = (expected_per_worker as f64 * 0.15) as usize; // 15% tolerance for least connections
    
    for worker_idx in 0..3 {
        let count = distribution.get(&worker_idx).copied().unwrap_or(0);
        let diff = if count > expected_per_worker {
            count - expected_per_worker
        } else {
            expected_per_worker - count
        };
        assert!(
            diff <= tolerance,
            "Worker {} received {} requests, expected ~{} (tolerance: {})",
            worker_idx, count, expected_per_worker, tolerance
        );
    }

    // Should still work correctly after concurrent access
    assert!(lc.select_worker(3).unwrap() < 3);
}