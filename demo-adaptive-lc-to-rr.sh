#!/bin/bash
# demo-adaptive-lc-to-rr.sh
# Demonstrates adaptive load balancing: Least Connections → Round Robin
# Shows automatic strategy switching when high error rate is detected

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Trap Ctrl+C for cleanup
trap cleanup INT TERM

cleanup() {
    echo ""
    echo -e "${YELLOW}🛑 Stopping all processes...${NC}"
    pkill -f 'load_balancer --config' 2>/dev/null || true
    pkill -f 'worker --port' 2>/dev/null || true
    sleep 1
    echo -e "${GREEN}✅ Cleanup complete${NC}"
    exit 0
}

#=============================================================================
# PHASE 1: INTRODUCTION AND SETUP
#=============================================================================

clear
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}║   ${CYAN}🎬 ADAPTIVE LOAD BALANCING: LEAST CONNECTIONS → ROUND ROBIN${BLUE}     ║${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}This demonstration will showcase:${NC}"
echo -e "  ${GREEN}✓${NC} Starting with least_connections strategy"
echo -e "  ${GREEN}✓${NC} Workers with high error rates (>10%)"
echo -e "  ${GREEN}✓${NC} Automatic switch to round_robin when majority of workers fail"
echo -e "  ${GREEN}✓${NC} Real-time metrics monitoring"
echo -e "  ${GREEN}✓${NC} Admin API endpoints"
echo ""
echo -e "${YELLOW}Press Enter to start...${NC}"
read

echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 1: Setup & Initialization${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

# Check dependencies
echo -e "${CYAN}Checking dependencies...${NC}"

if ! command -v jq &> /dev/null; then
    echo -e "${RED}✗ jq not found${NC}"
    echo -e "${YELLOW}  Please install: sudo apt install jq${NC}"
    exit 1
fi
echo -e "${GREEN}✓ jq available${NC}"

if ! command -v curl &> /dev/null; then
    echo -e "${RED}✗ curl not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ curl available${NC}"

# Cleanup previous processes
echo ""
echo -e "${CYAN}Cleaning up any previous processes...${NC}"
pkill -f 'load_balancer --config' 2>/dev/null || true
pkill -f 'worker --port' 2>/dev/null || true
sleep 1
echo -e "${GREEN}✓ Clean slate${NC}"

# Build
echo ""
echo -e "${CYAN}Building binaries (this may take a moment)...${NC}"
cargo build --bin worker --bin load_balancer --quiet 2>&1 | grep -v "Compiling" | grep -v "Finished" || true
echo -e "${GREEN}✓ Build complete${NC}"

echo ""
sleep 3

#=============================================================================
# PHASE 2: STARTING WORKERS FROM TOML
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 2: Starting Workers from Configuration${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Using configuration file:${NC} ${GREEN}config.lc-to-rr.toml${NC}"
echo ""

echo -e "${CYAN}Starting workers with start-workers.sh...${NC}"
echo ""
sleep 3

# Run start-workers.sh (suppress TOML parsing message)  
./start-workers.sh config.lc-to-rr.toml | grep -v "Using basic TOML parsing"

echo ""
echo -e "${CYAN}Waiting for workers to be ready...${NC}"
sleep 3

# Health check each worker
echo ""
echo -e "${CYAN}Verifying workers are responding (curl /test):${NC}"
echo ""

WORKERS=(3001 3002 3003)
WORKER_STATUS=()

for PORT in "${WORKERS[@]}"; do
    # Try up to 3 times for workers with artificial errors
    SUCCESS=false
    for attempt in {1..3}; do
        RESPONSE=$(curl -s --max-time 2 http://localhost:$PORT/test 2>&1)
        if echo "$RESPONSE" | grep -q "worker on port"; then
            echo -e "  ${GREEN}✓${NC} Worker $PORT: ${CYAN}$(echo "$RESPONSE" | jq -r '.message' 2>/dev/null || echo "OK")${NC}"
            WORKER_STATUS+=("OK")
            SUCCESS=true
            break
        fi
        sleep 0.5
    done
    
    if [ "$SUCCESS" = false ]; then
        echo -e "  ${RED}✗${NC} Worker on port $PORT: ${RED}NOT RESPONDING (tried 3 times)${NC}"
        WORKER_STATUS+=("FAILED")
        echo ""
        echo -e "${RED}Worker failed to respond after retries. Exiting.${NC}"
        cleanup
        exit 1
    fi
done
sleep 5

# Show configuration summary
echo ""
echo -e "${CYAN}Worker Configuration Summary:${NC}"
echo -e "${BLUE}┌─────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│${NC} Port  │ Status   │ Delay    │ Error Rate │ Purpose          ${BLUE}│${NC}"
echo -e "${BLUE}├─────────────────────────────────────────────────────────────────┤${NC}"
echo -e "${BLUE}│${NC} 3001  │ ${YELLOW}Faulty${NC}   │ 300ms    │ 20%        │ High Error Rate  ${BLUE}│${NC}"
echo -e "${BLUE}│${NC} 3002  │ ${YELLOW}Faulty${NC}   │ 300ms    │ 15%        │ High Error Rate  ${BLUE}│${NC}"
echo -e "${BLUE}│${NC} 3003  │ ${GREEN}Fast${NC}     │ 300ms    │ 0%         │ Baseline         ${BLUE}│${NC}"
echo -e "${BLUE}└─────────────────────────────────────────────────────────────────┘${NC}"

echo ""
sleep 6

#=============================================================================
# PHASE 3: STARTING LOAD BALANCER
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 3: Starting Load Balancer${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Starting load balancer with config.lc-to-rr.toml...${NC}"
echo -e "${YELLOW}(Logs being written to /tmp/lb_demo.log)${NC}"
echo ""

# Start load balancer in background
cargo run --bin load_balancer --quiet -- --config config.lc-to-rr.toml > /tmp/lb_demo.log 2>&1 &
LB_PID=$!

echo -e "${CYAN}Waiting for load balancer to initialize...${NC}"
sleep 5

# Verify load balancer is running
if ! curl -s --max-time 2 http://localhost:8080/admin/strategy > /dev/null 2>&1; then
    echo -e "${RED}✗ Load balancer failed to start!${NC}"
    echo ""
    echo -e "${YELLOW}Last 20 lines of log:${NC}"
    tail -20 /tmp/lb_demo.log
    cleanup
    exit 1
fi

echo -e "${GREEN}✓ Load balancer is running${NC}"
echo ""

# Display configuration
echo -e "${CYAN}Load Balancer Configuration:${NC}"
echo ""

STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy // "unknown"')
DECISION_STATUS=$(curl -s http://localhost:8080/admin/decision-status 2>/dev/null)
ADAPTIVE_ENABLED=$(echo "$DECISION_STATUS" | jq -r '.adaptive_enabled // false')
COOLDOWN=$(echo "$DECISION_STATUS" | jq -r '.cooldown_seconds // 60')

echo -e "  ${CYAN}●${NC} Initial Strategy: ${GREEN}$STRATEGY${NC}"
echo -e "  ${CYAN}●${NC} Adaptive Mode: ${GREEN}$ADAPTIVE_ENABLED${NC}"
echo -e "  ${CYAN}●${NC} Decision Thresholds:"
echo -e "      - High Latency: ${YELLOW}>400ms${NC}"
echo -e "      - High Error Rate: ${YELLOW}>10%${NC}"
echo -e "      - Min Samples: ${YELLOW}10 requests${NC}"
echo -e "  ${CYAN}●${NC} Cooldown Period: ${YELLOW}${COOLDOWN} seconds${NC}"
echo ""
sleep 5

# Test Admin API endpoints
echo -e "${CYAN}Testing Admin API endpoints:${NC}"
echo ""

echo -e "${YELLOW}1. GET /admin/strategy${NC}"
STRATEGY_RESPONSE=$(curl -s http://localhost:8080/admin/strategy 2>&1)
if echo "$STRATEGY_RESPONSE" | jq . > /dev/null 2>&1; then
    echo "$STRATEGY_RESPONSE" | jq .
    echo -e "  ${GREEN}✓${NC} Endpoint working"
else
    echo -e "  ${RED}✗${NC} Failed"
fi
sleep 2

echo ""
echo -e "${YELLOW}2. GET /admin/decision-status${NC}"
DECISION_RESPONSE=$(curl -s http://localhost:8080/admin/decision-status 2>&1)
if echo "$DECISION_RESPONSE" | jq . > /dev/null 2>&1; then
    echo "$DECISION_RESPONSE" | jq .
    echo -e "  ${GREEN}✓${NC} Endpoint working"
else
    echo -e "  ${RED}✗${NC} Failed"
fi
sleep 5

echo ""
echo -e "${YELLOW}3. GET /admin/metrics${NC}"
METRICS_RESPONSE=$(curl -s http://localhost:8080/admin/metrics 2>&1)
if echo "$METRICS_RESPONSE" | jq . > /dev/null 2>&1; then
    echo "$METRICS_RESPONSE" | jq '{workers: (.workers | map({url: .worker_url, requests: .total_requests, errors: .error_rate, latency: .average_latency_ms}))}'
    echo -e "  ${GREEN}✓${NC} Endpoint working"
else
    echo -e "  ${RED}✗${NC} Failed"
fi
sleep 5

echo ""
echo -e "${GREEN}✅ Phase 1-3 Complete!${NC}"
echo ""
sleep 5
echo -e "${YELLOW}System is ready for traffic.${NC}"
echo -e "${YELLOW}Press Enter to continue to Phase 4 (Traffic Generation)...${NC}"
read

#=============================================================================
# PHASE 4: TRAFFIC GENERATION AND MONITORING
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 4: Traffic Generation & Adaptive Monitoring${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Generating traffic to trigger adaptive behavior...${NC}"
echo -e "${YELLOW}Target: Generate enough requests to trigger high error rate detection${NC}"
echo -e "${YELLOW}Expected: Switch from least_connections → round_robin (need majority >10% errors)${NC}"
echo -e "${YELLOW}Note: Decision engine evaluates every 5 seconds${NC}"
echo ""
sleep 3

# Generate traffic in background (150 requests, 0.2s interval = ~30 seconds)
echo -e "${CYAN}Sending 150 requests (0.2s interval, ~30 seconds)...${NC}"
(
    for i in {1..150}; do
        curl -s http://localhost:8080/test > /dev/null 2>&1 &
        sleep 0.2
    done
) &
TRAFFIC_PID=$!

echo ""
echo -e "${CYAN}Monitoring system in real-time (waiting for decision engine evaluation)...${NC}"
echo ""
sleep 2

# Monitor for up to 35 seconds showing metrics every 5 seconds
INITIAL_STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy')
echo -e "${MAGENTA}Initial Strategy: ${INITIAL_STRATEGY}${NC}"
echo ""

STRATEGY_CHANGED=false
for i in {1..7}; do
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    echo -e "${CYAN}📊 Metrics Check ${i}/7 (every 5s to align with decision engine)${NC}"
    echo ""
    
    # Get current metrics
    METRICS=$(curl -s http://localhost:8080/admin/metrics 2>/dev/null)
    DECISION=$(curl -s http://localhost:8080/admin/decision-status 2>/dev/null)
    CURRENT_STRATEGY=$(echo "$DECISION" | jq -r '.current_strategy')
    
    # Display worker metrics
    echo "$METRICS" | jq -r '.workers[] | "  Worker \(.worker_url | split(":")[2]): \(.total_requests) req | \(.error_rate | floor)% err | \(.average_latency_ms | floor)ms avg"'
    
    echo ""
    echo -e "  ${CYAN}Current Strategy:${NC} ${GREEN}$CURRENT_STRATEGY${NC}"
    
    # Check if strategy changed
    if [ "$CURRENT_STRATEGY" != "$INITIAL_STRATEGY" ] && [ "$STRATEGY_CHANGED" = "false" ]; then
        # Infer reason based on destination strategy
        if [ "$CURRENT_STRATEGY" = "least_connections" ]; then
            REASON="high_latency"
        else
            REASON="high_error_rate"
        fi
        
        echo ""
        echo -e "${GREEN}🔄 STRATEGY CHANGED!${NC}"
        echo -e "   ${YELLOW}$INITIAL_STRATEGY${NC} → ${GREEN}$CURRENT_STRATEGY${NC}"
        echo -e "   Reason: ${MAGENTA}$REASON${NC}"
        STRATEGY_CHANGED=true
    fi
    
    echo ""
    sleep 5
done

# Wait for traffic generation to complete
wait $TRAFFIC_PID 2>/dev/null

echo ""
echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
echo ""

# Final metrics
echo -e "${CYAN}Final Metrics:${NC}"
echo ""
FINAL_METRICS=$(curl -s http://localhost:8080/admin/metrics 2>/dev/null)
echo "$FINAL_METRICS" | jq '.workers[] | {url: .worker_url, total_requests, successful_requests, failed_requests, avg_latency_ms: .average_latency_ms, error_rate}'

echo ""
FINAL_DECISION=$(curl -s http://localhost:8080/admin/decision-status 2>/dev/null)
FINAL_STRATEGY=$(echo "$FINAL_DECISION" | jq -r '.current_strategy')

# Get total metrics for summary
TOTAL_REQUESTS=$(echo "$FINAL_METRICS" | jq -r '.total_requests')
OVERALL_SUCCESS=$(echo "$FINAL_METRICS" | jq -r '.overall_success_rate | floor')
AVG_LATENCY=$(echo "$FINAL_METRICS" | jq '[.workers[].average_latency_ms] | add / length | floor')

if [ "$FINAL_STRATEGY" != "$INITIAL_STRATEGY" ]; then
    # Infer reason based on destination strategy
    if [ "$FINAL_STRATEGY" = "least_connections" ]; then
        SWITCH_REASON="High Latency Detected (>300ms on majority of workers)"
    else
        SWITCH_REASON="High Error Rate Detected (>10% on majority of workers)"
    fi
    
    echo -e "${GREEN}✅ Adaptive behavior demonstrated successfully!${NC}"
    echo -e "   Strategy switched from ${YELLOW}$INITIAL_STRATEGY${NC} to ${GREEN}$FINAL_STRATEGY${NC}"
    echo -e "   ${CYAN}Total: ${TOTAL_REQUESTS} requests | ${OVERALL_SUCCESS}% success | ${AVG_LATENCY}ms avg latency${NC}"
    LAST_EVAL=$(echo "$FINAL_DECISION" | jq -r '.last_evaluation')
    echo -e "   Last evaluation: ${CYAN}$LAST_EVAL${NC}"
    echo -e "   Reason: ${MAGENTA}$SWITCH_REASON${NC}"
else
    echo -e "${YELLOW}ℹ️  Strategy remained: $FINAL_STRATEGY${NC}"
    echo -e "   ${CYAN}Total: ${TOTAL_REQUESTS} requests | ${OVERALL_SUCCESS}% success | ${AVG_LATENCY}ms avg latency${NC}"
    echo -e "   (Thresholds not met: need majority of workers >10% errors or >400ms latency)${NC}"
fi

echo ""
sleep 5
echo -e "${YELLOW}Press Enter to continue to Phase 5 (Admin API Demo)...${NC}"
read
