# Load Balancer

An HTTP load balancer implemented in Rust with runtime strategy switching, comprehensive error handling, and observability. Built with modern async Rust using `hyper`, `tokio`, and following domain-driven design principles.

## ✨ Features

### Load Balancing Strategies
- **Round Robin**: Sequential distribution across workers with atomic operations
- **Least Connections**: Routes to worker with fewest active connections
- **Runtime Switching**: Change strategies without service restart via Admin API

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
curl http://127.0.0.1:1337/health
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

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

The project includes:
- **Unit tests**: Algorithm correctness, validation logic
- **Integration tests**: End-to-end strategy behavior
- **Concurrency tests**: Thread-safety verification
- **Doc tests**: Example code validation

### Manual Testing Scripts

Test scripts are available for validating load balancing behavior:

```bash
# Test Round Robin distribution
./test_round_robin.sh

# Test Least Connections under load
./test_least_connections.sh
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

## 🔮 Future Extensions

The modular architecture enables easy addition of:

- **New algorithms**: Weighted Round Robin, IP Hash, Consistent Hashing
- **Health checking**: Active/passive health monitoring
- **Metrics & telemetry**: Prometheus integration, request latency tracking
- **Configuration hot-reload**: Watch TOML file for changes
- **Connection pooling**: Reuse connections to workers
- **Circuit breakers**: Automatic failure detection and recovery

## 📝 License

This project is part of the Rust Bootcamp learning materials.

## 🙏 Acknowledgments

Built with feedback from the Rust community, incorporating modern patterns for error handling, concurrency, and observability.
