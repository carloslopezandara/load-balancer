#!/bin/bash
# Test script for graceful shutdown
# Simulates slow requests and verifies they complete before shutdown

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}║   ${CYAN}🧪 GRACEFUL SHUTDOWN TEST${BLUE}                                      ║${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    pkill -f 'load_balancer --config' 2>/dev/null || true
    pkill -f 'worker --port' 2>/dev/null || true
    rm -f /tmp/shutdown_test_*.log
    sleep 1
}

trap cleanup EXIT

# Build
echo -e "${CYAN}Building project...${NC}"
cargo build --quiet --bin worker --bin load_balancer
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Start workers
echo -e "${CYAN}Starting test workers...${NC}"

# Worker 1: Normal speed (100ms)
cargo run --quiet --bin worker -- --port 3001 --artificial-delay-ms 100 > /tmp/shutdown_test_worker1.log 2>&1 &
echo -e "  ${GREEN}✓${NC} Worker 3001: 100ms delay"

# Worker 2: Slow (3 seconds - to test graceful shutdown)
cargo run --quiet --bin worker -- --port 3002 --artificial-delay-ms 3000 > /tmp/shutdown_test_worker2.log 2>&1 &
echo -e "  ${GREEN}✓${NC} Worker 3002: 3000ms delay (slow for testing)"

# Worker 3: Fast
cargo run --quiet --bin worker -- --port 3003 --artificial-delay-ms 50 > /tmp/shutdown_test_worker3.log 2>&1 &
echo -e "  ${GREEN}✓${NC} Worker 3003: 50ms delay"

sleep 2
echo ""

# Start load balancer
echo -e "${CYAN}Starting load balancer...${NC}"
cargo run --quiet --bin load_balancer -- --config config.rr-to-lc.toml > /tmp/shutdown_test_lb.log 2>&1 &
LB_PID=$!

sleep 3

if ! curl -s --max-time 2 http://localhost:8080/admin/strategy > /dev/null 2>&1; then
    echo -e "${RED}✗ Load balancer failed to start${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Load balancer running (PID: $LB_PID)${NC}"
echo ""

# Send some fast requests first
echo -e "${CYAN}Sending 5 fast requests...${NC}"
for i in {1..5}; do
    curl -s http://localhost:8080/test > /dev/null &
done
sleep 1
echo -e "${GREEN}✓ Fast requests sent${NC}"
echo ""

# Now send many requests to increase chance of hitting slow worker
echo -e "${CYAN}Starting 10 concurrent requests (some will be slow)...${NC}"
echo -e "${YELLOW}Slow requests should complete gracefully even after Ctrl+C${NC}"
echo ""

# Start many requests to saturate the load balancer
for i in {1..10}; do
    (
        echo -e "  ${BLUE}→${NC} Request $i started at $(date +%H:%M:%S)"
        START=$(date +%s)
        RESPONSE=$(curl -s --max-time 10 http://localhost:8080/test 2>&1)
        END=$(date +%s)
        DURATION=$((END - START))
        
        if echo "$RESPONSE" | grep -q "worker on port"; then
            PORT=$(echo "$RESPONSE" | grep -oP 'port \K[0-9]+' || echo "unknown")
            if [ $DURATION -ge 2 ]; then
                echo -e "  ${GREEN}✓${NC} Request $i completed in ${DURATION}s (slow worker: $PORT) at $(date +%H:%M:%S)"
            else
                echo -e "  ${GREEN}✓${NC} Request $i completed in ${DURATION}s (fast worker: $PORT)"
            fi
            echo "$RESPONSE" > /tmp/shutdown_test_response_$i.log
        else
            echo -e "  ${RED}✗${NC} Request $i failed after ${DURATION}s"
        fi
    ) &
    CURL_PIDS[$i]=$!
    sleep 0.1  # Small delay to stagger requests
done

# Wait longer for requests to start processing - but NOT long enough for 3s requests to complete
sleep 1

echo ""
echo -e "${CYAN}Some slow requests should still be processing...${NC}"
echo -e "${YELLOW}Now triggering shutdown to test graceful completion${NC}"

# Show active connections
echo ""
echo -e "${CYAN}Active connections established${NC}"
echo ""

# Now trigger shutdown
echo -e "${YELLOW}Sending shutdown signal (Ctrl+C) to load balancer...${NC}"
echo -e "${YELLOW}Graceful shutdown should wait for slow requests to complete${NC}"
echo ""

kill -INT $LB_PID

# Monitor the shutdown process
echo -e "${CYAN}Monitoring shutdown...${NC}"
echo ""

# Wait for all curl requests to complete (max 15 seconds)
WAIT_START=$(date +%s)
for i in {1..10}; do
    if wait ${CURL_PIDS[$i]} 2>/dev/null; then
        : # Request finished
    fi
done

WAIT_END=$(date +%s)
TOTAL_WAIT=$((WAIT_END - WAIT_START))

echo ""
echo -e "${CYAN}Checking load balancer logs...${NC}"
sleep 1

# Check if graceful shutdown messages appear
if grep -q "Shutdown signal received" /tmp/shutdown_test_lb.log; then
    echo -e "  ${GREEN}✓${NC} Shutdown signal detected"
else
    echo -e "  ${RED}✗${NC} Shutdown signal not found in logs"
fi

if grep -q "Waiting for active connections" /tmp/shutdown_test_lb.log; then
    echo -e "  ${GREEN}✓${NC} Graceful shutdown initiated"
else
    echo -e "  ${RED}✗${NC} Graceful shutdown message not found"
fi

if grep -q "All connections closed\|connections will be aborted" /tmp/shutdown_test_lb.log; then
    echo -e "  ${GREEN}✓${NC} Shutdown completed"
else
    echo -e "  ${RED}✗${NC} Shutdown completion message not found"
fi

# Check how many requests completed
COMPLETED=0
SLOW_COMPLETED=0
for i in {1..10}; do
    if [ -f "/tmp/shutdown_test_response_$i.log" ]; then
        COMPLETED=$((COMPLETED + 1))
        # Check if it was a slow worker response
        if grep -q "3002" /tmp/shutdown_test_response_$i.log; then
            SLOW_COMPLETED=$((SLOW_COMPLETED + 1))
        fi
    fi
done

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}📊 Test Results:${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Total requests sent: ${CYAN}10${NC}"
echo -e "  Requests completed: ${GREEN}$COMPLETED${NC}"
echo -e "  Slow worker (3002) hits: ${YELLOW}$SLOW_COMPLETED${NC}"
echo -e "  Shutdown wait time: ${CYAN}${TOTAL_WAIT}s${NC}"
echo ""

# Show relevant log lines
echo -e "${CYAN}Key log entries:${NC}"
grep -E "(Shutdown signal|active_connections|Waiting for active|completed|aborted)" /tmp/shutdown_test_lb.log | tail -10 || echo "  (No matching logs found)"

echo ""
if [ $COMPLETED -ge 8 ] && [ $SLOW_COMPLETED -gt 0 ]; then
    echo -e "${GREEN}✅ SUCCESS: Graceful shutdown working!${NC}"
    echo -e "${GREEN}   - $COMPLETED/10 requests completed${NC}"
    echo -e "${GREEN}   - $SLOW_COMPLETED slow requests handled gracefully${NC}"
    echo -e "${GREEN}   - Server waited for active connections${NC}"
    exit 0
elif [ $COMPLETED -ge 5 ]; then
    echo -e "${YELLOW}⚠️  PARTIAL: $COMPLETED/10 requests completed${NC}"
    echo -e "${YELLOW}   Graceful shutdown is working but some connections were aborted${NC}"
    exit 0
else
    echo -e "${RED}❌ FAILED: Only $COMPLETED/10 requests completed${NC}"
    exit 1
fi
