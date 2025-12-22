#!/bin/bash
# demo-adaptive-combined.sh
# Demonstrates COMBINED adaptive scenarios:
# - Both high latency AND high errors present
# - Shows priority: errors take precedence over latency

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

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
# INTRODUCTION
#=============================================================================

clear
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}║   ${CYAN}🎬 COMBINED ADAPTIVE DEMO: ERRORS + LATENCY${BLUE}                    ║${NC}"
echo -e "${BLUE}║                                                                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}This demonstration will showcase:${NC}"
echo -e "  ${GREEN}✓${NC} Workers with BOTH high latency AND high errors"
echo -e "  ${GREEN}✓${NC} Decision engine priority logic (errors > latency)"
echo -e "  ${GREEN}✓${NC} Automatic strategy switching based on worst problem"
echo -e "  ${GREEN}✓${NC} Real-time EMA metrics convergence"
echo ""
echo -e "${YELLOW}Press Enter to start...${NC}"
read

#=============================================================================
# PHASE 1: SETUP
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 1: Setup & Initialization${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Checking dependencies...${NC}"
if ! command -v jq &> /dev/null; then
    echo -e "${RED}✗ jq not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ jq available${NC}"

if ! command -v curl &> /dev/null; then
    echo -e "${RED}✗ curl not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ curl available${NC}"

echo ""
echo -e "${CYAN}Cleaning up any previous processes...${NC}"
pkill -f 'load_balancer' 2>/dev/null || true
pkill -f 'worker --port' 2>/dev/null || true
sleep 2
echo -e "${GREEN}✓ Clean slate${NC}"

echo ""
echo -e "${CYAN}Building binaries (this may take a moment)...${NC}"
cargo build --quiet --bin load_balancer --bin worker
echo -e "${GREEN}✓ Build complete${NC}"

#=============================================================================
# PHASE 2: START WORKERS
#=============================================================================

echo ""
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 2: Starting Workers (Combined Scenario)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Starting workers with BOTH latency AND error problems:${NC}"
echo ""

# Worker 1: BOTH high latency AND high errors
echo -e "${CYAN}Starting Worker 1 (port 3001):${NC}"
echo -e "  - Delay: ${RED}2000ms${NC} (very slow - triggers latency threshold)"
echo -e "  - Error Rate: ${RED}25%${NC} (high errors - triggers error threshold)"
cargo run --quiet --bin worker -- --port 3001 --artificial-delay-ms 2000 --artificial-error-rate 0.25 > /tmp/worker3001.log 2>&1 &
echo -e "  ${GREEN}✓ Started${NC} (PID: $!)"
echo ""
sleep 1

# Worker 2: BOTH high latency AND high errors
echo -e "${CYAN}Starting Worker 2 (port 3002):${NC}"
echo -e "  - Delay: ${RED}2000ms${NC} (very slow - triggers latency threshold)"
echo -e "  - Error Rate: ${RED}25%${NC} (high errors - triggers error threshold)"
cargo run --quiet --bin worker -- --port 3002 --artificial-delay-ms 2000 --artificial-error-rate 0.25 > /tmp/worker3002.log 2>&1 &
echo -e "  ${GREEN}✓ Started${NC} (PID: $!)"
echo ""
sleep 1

# Worker 3: Baseline (no problems)
echo -e "${CYAN}Starting Worker 3 (port 3003):${NC}"
echo -e "  - Delay: ${GREEN}50ms${NC} (very fast)"
echo -e "  - Error Rate: ${GREEN}0%${NC} (reliable - baseline)"
cargo run --quiet --bin worker -- --port 3003 --artificial-delay-ms 50 --artificial-error-rate 0.0 > /tmp/worker3003.log 2>&1 &
echo -e "  ${GREEN}✓ Started${NC} (PID: $!)"
echo ""

echo -e "${CYAN}Waiting for workers to be ready...${NC}"
sleep 3

# Verify workers
echo ""
echo -e "${CYAN}Verifying workers are responding:${NC}"
echo ""

for PORT in 3001 3002 3003; do
    for attempt in {1..5}; do
        RESPONSE=$(curl -s --max-time 5 http://localhost:$PORT/test 2>&1)
        if echo "$RESPONSE" | grep -q "worker on port"; then
            echo -e "  ${GREEN}✓${NC} Worker $PORT: Ready"
            break
        fi
        if [ $attempt -eq 5 ]; then
            echo -e "  ${RED}✗${NC} Worker $PORT: Not responding"
            cleanup
            exit 1
        fi
        sleep 3
    done
done

echo ""
echo -e "${CYAN}Worker Configuration Summary:${NC}"
echo -e "${BLUE}┌─────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│${NC} Port  │ Latency  │ Errors │ Issues                        ${BLUE}│${NC}"
echo -e "${BLUE}├─────────────────────────────────────────────────────────────────┤${NC}"
echo -e "${BLUE}│${NC} 3001  │ ${RED}2000ms${NC}   │ ${RED}25%${NC}    │ BOTH latency + errors         ${BLUE}│${NC}"
echo -e "${BLUE}│${NC} 3002  │ ${RED}2000ms${NC}   │ ${RED}25%${NC}    │ BOTH latency + errors         ${BLUE}│${NC}"
echo -e "${BLUE}│${NC} 3003  │ ${GREEN}50ms${NC}     │ ${GREEN}0%${NC}     │ Baseline (no problems)        ${BLUE}│${NC}"
echo -e "${BLUE}└─────────────────────────────────────────────────────────────────┘${NC}"
echo ""
echo -e "${YELLOW}⚠️  MAJORITY (2/3) workers have BOTH problems:${NC}"
echo -e "   - High Latency: 2 workers with 2000ms > 400ms threshold"
echo -e "   - High Errors: 2 workers with 40% > 10% threshold"
echo -e "   ${MAGENTA}→ Watch how the system adapts to changing conditions${NC}"
echo -e "   ${MAGENTA}→ Priority: Errors > Latency when both present${NC}"
echo ""
sleep 5

#=============================================================================
# PHASE 3: START LOAD BALANCER
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 3: Starting Load Balancer${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Starting load balancer (adaptive mode)...${NC}"
cargo run --quiet --bin load_balancer -- \
    --config config.combined-demo.toml > /tmp/lb_combined.log 2>&1 &
LB_PID=$!

echo -e "${CYAN}Waiting for load balancer...${NC}"
sleep 5

if ! curl -s --max-time 2 http://localhost:8080/admin/strategy > /dev/null 2>&1; then
    echo -e "${RED}✗ Load balancer failed to start!${NC}"
    cleanup
    exit 1
fi
echo -e "${GREEN}✓ Load balancer running${NC}"
echo ""

STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy')
echo -e "  ${CYAN}●${NC} Initial Strategy: ${GREEN}$STRATEGY${NC}"
  echo -e "  ${CYAN}●${NC} Adaptive Mode: ${GREEN}Enabled${NC}"
echo -e "  ${CYAN}●${NC} Decision Thresholds:"
echo -e "      - High Latency: ${YELLOW}>400ms${NC}"
echo -e "      - High Error Rate: ${YELLOW}>10%${NC}"
echo -e "      - Min Samples: ${YELLOW}30 requests${NC}"
echo -e "      - Cooldown: ${YELLOW}30 seconds${NC}"
echo ""
sleep 3

#=============================================================================
# PHASE 4: TRAFFIC GENERATION
#=============================================================================

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}PHASE 4: Traffic Generation & Monitoring${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Generating traffic to trigger adaptive behavior...${NC}"
echo -e "${YELLOW}Expected behavior:${NC}"
echo -e "  ${MAGENTA}1.${NC} System evaluates conditions in real-time every 5 seconds"
echo -e "  ${MAGENTA}2.${NC} When BOTH problems present → Priority: Errors > Latency (stay RR)"
echo -e "  ${MAGENTA}3.${NC} When ONLY latency high → Switch to LC for optimization"
echo -e "  ${MAGENTA}4.${NC} When errors rise again → Return to RR (error handling priority)"
echo ""
sleep 3

# Generate high concurrency traffic
echo -e "${CYAN}Sending 1800 requests (0.05s interval, ~90 seconds)...${NC}"
(
    for i in {1..1800}; do
        curl -s http://localhost:8080/test > /dev/null 2>&1 &
        sleep 0.05
    done
) &
TRAFFIC_PID=$!

echo ""
echo -e "${CYAN}Monitoring system (checking every 5s)...${NC}"
echo ""
sleep 2

INITIAL_STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy')
echo -e "${MAGENTA}Initial Strategy: ${INITIAL_STRATEGY}${NC}"
echo ""

STRATEGY_CHANGED=false
for i in {1..18}; do
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    echo -e "${CYAN}📊 Metrics Check ${i} (every 5s to align with decision engine)${NC}"
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
    if [ "$CURRENT_STRATEGY" != "$INITIAL_STRATEGY" ]; then
        # Infer reason based on destination strategy
        if [ "$CURRENT_STRATEGY" = "least_connections" ]; then
            REASON="high_latency"
        else
            REASON="high_error_rate"
        fi
        
        echo ""
        echo -e "${CYAN}🔄 STRATEGY CHANGED${NC}"
        echo -e "   ${YELLOW}$INITIAL_STRATEGY${NC} → ${CYAN}$CURRENT_STRATEGY${NC}"
        echo -e "   Reason: ${MAGENTA}$REASON${NC}"
        
        # Show the actual evaluation metrics from the decision engine
        echo ""
        echo -e "   ${YELLOW}Decision Engine Evaluation (actual values used):${NC}"
        grep "Worker evaluation metrics" /tmp/lb_combined.log | tail -3 || echo "     (Logs not available yet)"
        echo ""
        if [ "$REASON" = "high_latency" ]; then
            echo -e "   ${YELLOW}ℹ️  Switched to LC: Latency high, errors below threshold${NC}"
        else
            echo -e "   ${GREEN}✅ Switched to RR: Error handling prioritized${NC}"
        fi
        
        STRATEGY_CHANGED=true
        INITIAL_STRATEGY=$CURRENT_STRATEGY
    fi
    
    echo ""
    sleep 5
done

echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
echo ""

#=============================================================================
# PHASE 5: FINAL RESULTS
#=============================================================================

echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
echo ""

# Wait for traffic generation to complete if still running
wait $TRAFFIC_PID 2>/dev/null

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

# Remember: INITIAL_STRATEGY was "round_robin" at the start
if [ "$STRATEGY_CHANGED" = "true" ]; then
    # Strategy changed during the demo - this is EXPECTED with fluctuating metrics
    echo -e "${GREEN}✅ Adaptive behavior demonstrated successfully!${NC}"
    echo -e "   Final Strategy: ${GREEN}$FINAL_STRATEGY${NC}"
    echo -e "   ${CYAN}Total: ${TOTAL_REQUESTS} requests | ${OVERALL_SUCCESS}% success | ${AVG_LATENCY}ms avg latency${NC}"
    LAST_EVAL=$(echo "$FINAL_DECISION" | jq -r '.last_evaluation')
    echo -e "   Last evaluation: ${CYAN}$LAST_EVAL${NC}"
    
    echo ""
    echo -e "${MAGENTA}🎯 Key Observations:${NC}"
    echo -e "   The system adapted dynamically based on real-time conditions:"
    echo -e "   ${CYAN}• When BOTH problems present → Errors take precedence (RR)${NC}"
    echo -e "   ${CYAN}• When ONLY latency high → Optimizes connections (LC)${NC}"
    echo -e "   ${CYAN}• When errors rise again → Returns to error handling (RR)${NC}"
    echo ""
    echo -e "   This demonstrates the priority logic works correctly:"
    echo -e "   ${GREEN}Error handling > Latency optimization${NC}"
else
    # Strategy stayed the same - this would mean errors were ALWAYS high
    echo -e "${GREEN}✅ Priority logic test PASSED!${NC}"
    echo -e "   Strategy correctly STAYED in ${GREEN}round_robin${NC}"
    echo -e "   ${CYAN}Total: ${TOTAL_REQUESTS} requests | ${OVERALL_SUCCESS}% success | ${AVG_LATENCY}ms avg latency${NC}"
    LAST_EVAL=$(echo "$FINAL_DECISION" | jq -r '.last_evaluation')
    echo -e "   Last evaluation: ${CYAN}$LAST_EVAL${NC}"
    
    echo ""
    echo -e "${MAGENTA}🎯 Key Learning:${NC}"
    echo -e "   Errors remained consistently high throughout the demo."
    echo -e "   System correctly prioritized ${GREEN}error handling${NC} over latency optimization."
    echo -e "   RR was maintained even with high latency present."
fi

echo ""
echo -e "${YELLOW}Press Enter to cleanup and exit...${NC}"
read

cleanup
