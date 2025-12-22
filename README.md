# Load Balancer

HTTP load balancer in Rust with adaptive strategy switching based on real-time performance metrics.

## Features

- **Round Robin**: Sequential distribution across workers
- **Least Connections**: Routes to worker with fewest active connections
- **Adaptive Mode**: Automatic strategy switching based on latency/error rate thresholds
- **Admin API**: Runtime monitoring and manual control

## Quick Start

### Start Workers and Load Balancer

```bash
# Terminal 1: Start workers from config
./start-workers.sh config.rr-to-lc.toml

# Terminal 2: Start load balancer
cargo run -- --config config.rr-to-lc.toml

# Terminal 3: Test
curl http://localhost:8080/test
```

### CLI Options

```bash
cargo run -- --help
```

Key options:
- `--config <FILE>`: Load configuration from TOML file
- `--strategy <STRATEGY>`: Initial strategy (round_robin/least_connections)
- `--port <PORT>`: Load balancer port (default: 1337)
- `--worker <URL>`: Worker URLs (multiple)
- `--log-level <LEVEL>`: trace/debug/info/warn/error

## Adaptive Load Balancing

Automatically switches strategies based on real-time metrics:

- **Evaluation**: Every 5 seconds
- **Cooldown**: 60 seconds between switches
- **Thresholds**: High latency (>300-500ms), high error rate (>10%)
- **Logic**: Requires majority of workers (>50%) to exceed thresholds

**Switching Rules:**
- High Latency → Switch to `least_connections`
- High Error Rate → Switch to `round_robin`

### Configuration

```toml
[server]
adaptive = true  # Enable automatic switching
strategy = "round_robin"  # Initial strategy
```

### Admin API

```bash
# Get current strategy
curl http://127.0.0.1:1337/admin/strategy

# Switch strategy manually
curl -X POST http://127.0.0.1:1337/admin/strategy \
  -H "Content-Type: application/json" \
  -d '{"strategy":"least_connections"}'

# Get worker metrics
curl http://127.0.0.1:1337/admin/metrics

# Get decision engine status
curl http://127.0.0.1:1337/admin/decision-status
```

## Demos

Two comprehensive demos showcase adaptive switching:

### Demo 1: Round Robin → Least Connections
High latency triggers switch to LC.

```bash
./demo-adaptive-rr-to-lc.sh
```

- **Workers**: Fast(50ms), Slow(800ms), Medium(400ms+errors)
- **Trigger**: 2/3 workers >300ms latency
- **Result**: Routes traffic to fastest worker

### Demo 2: Least Connections → Round Robin
High error rate triggers switch to RR.

```bash
./demo-adaptive-lc-to-rr.sh
```

- **Workers**: Faulty(20%), Faulty(15%), Healthy(0%)
- **Trigger**: 2/3 workers >10% error rate
- **Result**: Distributes evenly to avoid sticky errors

## Testing

```bash
cargo test  # 75 tests: unit, integration, concurrency
```

## Architecture

- **Strategy Pattern**: Trait-based for easy extensibility
- **Lock-free**: AtomicUsize for connection counting
- **Error Handling**: Centralized `LoadBalancerError`, no panics
- **RwLock**: Only for strategy switching
