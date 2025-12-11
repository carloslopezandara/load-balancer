/// Integration tests for adaptive load balancing
/// 
/// These tests verify end-to-end integration between LoadBalancer, DecisionEngine,
/// and MetricsCollector working together in realistic scenarios.

use load_balancer::services::LoadBalancerService;
use load_balancer::domain::WorkerUrl;
use load_balancer::load_balancing_strategy::LoadBalancingStrategy;
use std::time::Duration;

/// Test complete adaptive cycle with LoadBalancerService integration
#[tokio::test]
async fn test_load_balancer_adaptive_integration() {
    // Setup workers
    let workers = vec![
        WorkerUrl::parse("http://localhost:3001".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3002".to_string()).unwrap(),
    ];
    
    let strategy = LoadBalancingStrategy::new_round_robin();
    
    // Create load balancer with adaptive mode enabled
    let lb = LoadBalancerService::with_adaptive(workers.clone(), strategy, true)
        .expect("should create load balancer");
    
    // Verify adaptive mode is enabled
    assert!(lb.is_adaptive(), "Adaptive mode should be enabled");
    
    // Verify initial strategy
    let initial_strategy = lb.get_strategy_name();
    assert_eq!(initial_strategy.await, "round_robin");
    
    // Simulate high latency on both workers
    let metrics = lb.metrics_collector();
    for _ in 0..20 {
        metrics.record_success(0, Duration::from_millis(700));
        metrics.record_success(1, Duration::from_millis(800));
    }
    
    // Trigger evaluation and adaptation
    lb.evaluate_and_adapt().await.expect("evaluation should succeed");
    
    // Verify strategy switched to least_connections
    let new_strategy = lb.get_strategy_name().await;
    assert_eq!(new_strategy, "least_connections", "Should switch to least_connections due to high latency");
    
    // Verify decision engine state
    let engine = lb.decision_engine().expect("should have decision engine");
    assert!(!engine.can_switch().unwrap(), "Should be in cooldown after switch");
    
    let cooldown = engine.cooldown_remaining_seconds().unwrap();
    assert!(cooldown.is_some(), "Should have cooldown remaining");
    assert!(cooldown.unwrap() <= 60, "Cooldown should be <= 60 seconds");
}

/// Test that non-adaptive LoadBalancer doesn't switch strategies
#[tokio::test]
async fn test_load_balancer_non_adaptive_no_switch() {
    let workers = vec![
        WorkerUrl::parse("http://localhost:3001".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3002".to_string()).unwrap(),
    ];
    
    let strategy = LoadBalancingStrategy::new_round_robin();
    
    // Create load balancer with adaptive mode DISABLED
    let lb = LoadBalancerService::with_adaptive(workers.clone(), strategy, false)
        .expect("should create load balancer");
    
    assert!(!lb.is_adaptive(), "Adaptive mode should be disabled");
    assert!(lb.decision_engine().is_none(), "Should not have decision engine");
    
    // Simulate high latency
    let metrics = lb.metrics_collector();
    for _ in 0..20 {
        metrics.record_success(0, Duration::from_millis(700));
        metrics.record_success(1, Duration::from_millis(800));
    }
    
    // Try to evaluate (should be no-op)
    lb.evaluate_and_adapt().await.expect("should succeed even without engine");
    
    // Verify strategy didn't change
    assert_eq!(lb.get_strategy_name().await, "round_robin", "Strategy should remain unchanged");
}

/// Test metrics collection integration with LoadBalancer
#[tokio::test]
async fn test_metrics_integration_with_load_balancer() {
    let workers = vec![
        WorkerUrl::parse("http://localhost:3001".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3002".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3003".to_string()).unwrap(),
    ];
    
    let strategy = LoadBalancingStrategy::new_round_robin();
    let lb = LoadBalancerService::new(workers.clone(), strategy)
        .expect("should create load balancer");
    
    // Verify metrics collector is initialized with correct worker count
    let metrics = lb.metrics_collector();
    assert_eq!(metrics.all_metrics().len(), 3, "Should have metrics for 3 workers");
    
    // Record mixed success and errors
    metrics.record_success(0, Duration::from_millis(100));
    metrics.record_success(0, Duration::from_millis(150));
    metrics.record_error(0, Duration::from_millis(200));
    
    metrics.record_success(1, Duration::from_millis(50));
    metrics.record_error(1, Duration::from_millis(100));
    
    metrics.record_success(2, Duration::from_millis(80));
    metrics.record_success(2, Duration::from_millis(90));
    
    // Verify metrics are tracked correctly
    let worker0 = metrics.get_worker_metrics(0).unwrap();
    assert_eq!(worker0.request_count(), 3);
    assert_eq!(worker0.error_count(), 1);
    
    let worker1 = metrics.get_worker_metrics(1).unwrap();
    assert_eq!(worker1.request_count(), 2);
    assert_eq!(worker1.error_count(), 1);
    
    // Verify error rate calculation
    let error_rate0 = metrics.error_rate(0);
    assert!((error_rate0 - 0.333).abs() < 0.01, "Error rate should be ~33%");
    
    let error_rate1 = metrics.error_rate(1);
    assert!((error_rate1 - 0.5).abs() < 0.01, "Error rate should be 50%");
}

/// Test worker_hosts accessor returns correct workers
#[tokio::test]
async fn test_load_balancer_worker_hosts_integration() {
    let workers = vec![
        WorkerUrl::parse("http://localhost:3001".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3002".to_string()).unwrap(),
    ];
    
    let strategy = LoadBalancingStrategy::new_round_robin();
    let lb = LoadBalancerService::new(workers.clone(), strategy)
        .expect("should create load balancer");
    
    let hosts = lb.worker_hosts();
    assert_eq!(hosts.len(), 2);
    assert_eq!(hosts[0].as_str(), "http://localhost:3001");
    assert_eq!(hosts[1].as_str(), "http://localhost:3002");
}

/// Test adaptive mode with custom thresholds configuration
#[tokio::test]
async fn test_load_balancer_with_custom_thresholds() {
    use load_balancer::domain::DecisionThresholds;
    
    let workers = vec![
        WorkerUrl::parse("http://localhost:3001".to_string()).unwrap(),
        WorkerUrl::parse("http://localhost:3002".to_string()).unwrap(),
    ];
    
    let strategy = LoadBalancingStrategy::new_round_robin();
    
    // Custom thresholds: lower latency threshold (300ms instead of 500ms)
    let thresholds = DecisionThresholds {
        high_latency_ms: 300,
        high_error_rate: 0.15,
        min_samples: 5,
    };
    
    let lb = LoadBalancerService::with_adaptive_config(
        workers,
        strategy,
        true,
        thresholds,
        30, // 30 second cooldown instead of 60
    ).expect("should create load balancer");
    
    assert!(lb.is_adaptive(), "Adaptive mode should be enabled");
    
    // Simulate latency between 300-500ms (would not trigger with default 500ms threshold)
    let metrics = lb.metrics_collector();
    for _ in 0..10 {
        metrics.record_success(0, Duration::from_millis(400));
        metrics.record_success(1, Duration::from_millis(450));
    }
    
    // With custom 300ms threshold, this should trigger switch
    lb.evaluate_and_adapt().await.expect("evaluation should succeed");
    
    let new_strategy = lb.get_strategy_name().await;
    assert_eq!(new_strategy, "least_connections", 
        "Should switch to least_connections with custom lower threshold");
}
