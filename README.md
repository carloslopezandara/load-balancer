# Load Balancer

An HTTP load balancer implemented in Rust with runtime strategy switching, comprehensive error handling, and observability. Built with modern async Rust using `hyper`, `tokio`, and following domain-driven design principles.

## ✨ Features

### Load Balancing Strategies
- **Round Robin**: Sequential distribution across workers with atomic operations
- **Least Connections**: Routes to worker with fewest active connections
- **Runtime Switching**: Change strategies without service restart via Admin API
- **Adaptive Load Balancing**: Automatic strategy switching based on real-time performance metrics

### Architecture & Design
- **Domain-Driven Design**: Clear separation of concerns (domain/, services/, routes/)
- **Strategy Pattern**: Trait-based design for easy algorithm extensibility
- **Centralized Error Handling**: Custom `LoadBalancerError` with proper propagation
- **Robust Error Handling**: No panics in service code, comprehensive logging, graceful error recovery

### Observability & Operations
- **Structured Logging**: Full `tracing` integration with configurable log levels
- **CLI Configuration**: Rich command-line interface with `clap`
- **Health Endpoints**: JSON responses for monitoring
- **Admin API**: Runtime configuration management

## 🚀 Quick Start

### Prerequisites

- Rust toolchain (stable) – install via [rustup](https://rustup.rs/)

### Running with Default Configuration

1. **Start worker servers:**

```bash
# Terminal 1: Start first worker
cargo run --bin worker -- 3000

# Terminal 2: Start second worker
cargo run --bin worker -- 3001
```

2. **Start the load balancer:**

```bash
# Terminal 3: Start load balancer with default settings
cargo run --bin load_balancer
```

3. **Send test requests:**

```bash
# Terminal 4: Test the load balancer
curl http://127.0.0.1:1337/test
```

### CLI Options

The load balancer supports comprehensive CLI configuration:

```bash
cargo run --bin load_balancer -- --help
```

**Available options:**
- `--strategy <STRATEGY>`: Initial strategy (`round_robin` or `least_connections`)
- `--port <PORT>`: Load balancer port (default: 1337)
- `--worker <URL>`: Worker URLs (can be specified multiple times)
- `--config <FILE>`: Load configuration from TOML file
- `--log-level <LEVEL>`: Log level (trace, debug, info, warn, error)

**Examples:**

```bash
# Start with Round Robin on port 8080
cargo run --bin load_balancer -- --strategy round_robin --port 8080

# Configure specific workers
cargo run --bin load_balancer -- \
  --worker http://localhost:3000 \
  --worker http://localhost:3001 \
  --worker http://localhost:3002

# Enable debug logging
cargo run --bin load_balancer -- --log-level debug
```

## 🤖 Adaptive Load Balancing

The load balancer features an intelligent adaptive mode that automatically switches strategies based on real-time performance metrics.

### How It Works

The **Decision Engine** continuously monitors worker performance and switches strategies when issues are detected:

- **Evaluation Interval**: Every 5 seconds
- **Cooldown Period**: 60 seconds between switches (prevents thrashing)
- **Detection Thresholds**:
  - High Latency: >500ms average response time
  - High Error Rate: >10% failed requests
  - Minimum Samples: 10 requests per worker

**Switching Logic**:
- **High Latency Detected** → Switch to `least_connections` (distribute load to faster workers)
- **High Error Rate Detected** → Switch to `round_robin` (avoid sticky problematic workers)
- **Requires Majority**: >50% of workers must exceed thresholds to trigger switch

### Configuration

Enable adaptive mode in `config.toml`:

```toml
[server]
adaptive = true  # Enable automatic strategy switching
strategy = "round_robin"  # Initial strategy
```

Or disable it for manual control:

```toml
[server]
adaptive = false  # Manual-only strategy switching via Admin API
```

### Admin API Endpoints

The load balancer provides REST endpoints for monitoring and manual control:

#### Get Current Strategy
```bash
curl http://127.0.0.1:1337/admin/strategy
# Response: {"strategy":"round_robin"}
```

#### Switch Strategy Manually
```bash
curl -X POST http://127.0.0.1:1337/admin/strategy \
  -H "Content-Type: application/json" \
  -d '{"strategy":"least_connections"}'
# Response: {"message":"Strategy switched to least_connections"}
```

#### Get Worker Metrics
```bash
curl http://127.0.0.1:1337/admin/metrics
# Response: {
#   "workers": [
#     {
#       "index": 0,
#       "url": "http://localhost:3000",
#       "total_requests": 150,
#       "error_count": 5,
#       "error_rate": 0.033,
#       "avg_latency_ms": 245
#     },
#     ...
#   ]
# }
```

#### Get Decision Engine Status
```bash
curl http://127.0.0.1:1337/admin/decision-status
# Response: {
#   "adaptive_enabled": true,
#   "current_strategy": "least_connections",
#   "can_switch": false,
#   "cooldown_remaining_seconds": 45,
#   "last_switch_reason": "HighLatency"
# }
```

### Testing Adaptive Behavior

#### Using Artificial Worker Conditions

Workers support artificial performance degradation for testing:

```bash
# Start worker with 800ms artificial delay
cargo run --bin worker -- 3000 --artificial-delay-ms 800

# Start worker with 20% artificial error rate
cargo run --bin worker -- 3001 --artificial-error-rate 0.2

# Combine both conditions
cargo run --bin worker -- 3002 --artificial-delay-ms 500 --artificial-error-rate 0.15
```

#### Automated Testing Script

The `test-adaptive.sh` script demonstrates the complete adaptive cycle:

```bash
./test-adaptive.sh
```

**What it does:**
1. Starts 3 workers (1 normal, 1 with high latency, 1 with high error rate)
2. Starts load balancer in adaptive mode
3. Generates 100 test requests
4. Monitors strategy changes in real-time
5. Displays final metrics

**Expected output:**
```
🚀 Starting adaptive load balancing test...
📊 Initial strategy: round_robin
⏳ Generating traffic (100 requests)...
🔄 STRATEGY CHANGED: round_robin → least_connections
✅ Test completed!
📈 Final metrics: [worker performance data]
```

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

The project includes **58 comprehensive tests**:
- **Unit tests**: Algorithm correctness, validation logic, decision engine
- **Integration tests**: End-to-end strategy behavior, adaptive switching
- **Concurrency tests**: Thread-safety verification
- **Doc tests**: Example code validation

### Manual Testing Scripts

Test scripts are available for validating load balancing behavior:

```bash
# Test Round Robin distribution
./test_round_robin.sh

# Test Least Connections under load
./test_least_connections.sh

# Test Adaptive switching with artificial conditions
./test-adaptive.sh
```

## 🏛️ Architecture Details

### Error Handling

The project uses a centralized error handling approach:

```rust
// All operations return Result with LoadBalancerError
pub type Result<T> = std::result::Result<T, LoadBalancerError>;

// Errors are properly logged and converted to HTTP responses
let response = match router.handle(req).await {
    Ok(response) => response,
    Err(e) => {
        tracing::error!("Request handling error: {}", e);
        http_utils::create_error_response(&e)
    }
};
```

**Benefits:**
- No panics in service code
- Comprehensive error logging
- Graceful degradation
- Proper HTTP status codes

### Lock Management

Critical improvement in concurrency handling:

- **RwLock**: Used only for strategy switching (rare operation)
- **AtomicUsize**: Lock-free counters for connection tracking
- **Locks released immediately**: Never held during I/O operations

```rust
// Lock is acquired, operation performed, and released immediately
pub async fn connection_started(&self, worker_index: usize) {
    self.strategy.read().await.connection_started(worker_index);
    // Lock released here, before any I/O
}
```

### Strategy Pattern Implementation

Each strategy implements the `Strategy` trait:

```rust
pub trait Strategy: Send + Sync {
    fn select_worker(&self, worker_count: usize) -> Result<usize>;
    fn connection_started(&self, worker_index: usize);
    fn connection_ended(&self, worker_index: usize);
    fn get_name(&self) -> &'static str;
}
```

**Adding new strategies:**
1. Create new file in `src/load_balancing_strategy/`
2. Implement `Strategy` trait
3. Add to `LoadBalancingStrategy` enum in `mod.rs`
4. Update `StrategyType` in `domain/strategy.rs`

## 📊 Performance Considerations

- **Lock-free operations**: AtomicUsize for connection counting
- **Zero-copy where possible**: String optimizations with `impl Into<String>`
- **Efficient strategy names**: `&'static str` instead of heap allocations
- **Bounded error handling**: Multi-level fallbacks prevent cascading failures

## 🎯 Use Cases

### When to Use Adaptive Mode

**Enable adaptive mode (`adaptive = true`)** when:
- Worker performance varies over time
- You want automatic recovery from degraded workers
- Traffic patterns are unpredictable
- You need hands-off operation

**Disable adaptive mode (`adaptive = false`)** when:
- You have specific strategy requirements
- Workers have consistent performance
- You prefer manual control via Admin API
- Testing specific algorithm behavior

### Complementary Features

Adaptive mode and Admin API work together:
- **Adaptive mode**: Automatic optimization for common issues
- **Admin API**: Emergency manual override and monitoring
- **Metrics endpoint**: Observe system state for debugging
- **Decision status**: Understand why switches occurred

## 🔮 Future Extensions

The modular architecture enables easy addition of:

- **New algorithms**: Weighted Round Robin, IP Hash, Consistent Hashing
- **Advanced health checking**: Active/passive health monitoring with circuit breakers
- **Custom thresholds**: Configurable latency/error rate limits per deployment
- **Metrics & telemetry**: Prometheus integration, detailed request latency tracking
- **Configuration hot-reload**: Watch TOML file for changes
- **Connection pooling**: Reuse connections to workers

## 📝 License

This project is part of the Rust Bootcamp learning materials.

## 🙏 Acknowledgments

Built with feedback from the Rust community, incorporating modern patterns for error handling, concurrency, and observability.
