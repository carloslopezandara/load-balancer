use load_balancer::load_balancing_strategy::round_robin::RoundRobin;
use load_balancer::load_balancing_strategy::traits::Strategy;
use std::sync::Arc;
use std::thread;
use std::collections::HashMap;

/// Test Round Robin distribution under heavy concurrent load
/// 
/// This integration test verifies thread-safety and that distribution 
/// remains balanced when multiple threads select workers simultaneously.
/// Goes beyond the basic unit test to verify statistical distribution.
#[test]
fn test_round_robin_concurrent_distribution() {
    let rr = Arc::new(RoundRobin::new());
    let worker_count = 3;
    let iterations = 300; // 10 threads * 300 = 3000 total selections
    let mut handles = vec![];
    
    // Shared counter to collect results
    let results = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Test concurrent access with multiple threads
    for _ in 0..10 {
        let rr_clone = Arc::clone(&rr);
        let results_clone = Arc::clone(&results);
        let handle = thread::spawn(move || {
            let mut local_results = Vec::new();
            for _ in 0..iterations {
                let worker = rr_clone.select_worker(worker_count).unwrap();
                local_results.push(worker);
            }
            results_clone.lock().unwrap().extend(local_results);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify distribution is balanced (within 5% tolerance)
    let selections = results.lock().unwrap();
    let mut distribution: HashMap<usize, usize> = HashMap::new();
    for &worker in selections.iter() {
        *distribution.entry(worker).or_insert(0) += 1;
    }
    
    let total = selections.len();
    let expected_per_worker = total / worker_count;
    let tolerance = (expected_per_worker as f64 * 0.05) as usize; // 5% tolerance
    
    for worker_idx in 0..worker_count {
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
    assert!(rr.select_worker(worker_count).unwrap() < worker_count);
}