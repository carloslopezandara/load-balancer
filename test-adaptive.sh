#!/bin/bash
# Test script for adaptive load balancing
# This script demonstrates how to trigger adaptive behavior

# Clean up any existing processes first
echo "🧹 Cleaning up any existing processes..."
pkill -9 -f "worker --port" 2>/dev/null
pkill -9 -f "load_balancer --config" 2>/dev/null
sleep 1

echo "🔨 Building workers (this may take a moment)..."
cargo build --bin worker --quiet 2>/dev/null

echo ""
echo "🧪 Starting workers with artificial conditions..."
echo ""

# Start worker 1: Normal behavior
echo "✅ Starting Worker 1 on port 3001 (normal)"
cargo run --bin worker --quiet -- --port 3001 > /tmp/worker1.log 2>&1 &
WORKER1_PID=$!

# Start worker 2: High latency
echo "⏱️  Starting Worker 2 on port 3002 (800ms delay)"
cargo run --bin worker --quiet -- --port 3002 --artificial-delay-ms 800 > /tmp/worker2.log 2>&1 &
WORKER2_PID=$!

# Start worker 3: High error rate
echo "❌ Starting Worker 3 on port 3003 (20% error rate)"
cargo run --bin worker --quiet -- --port 3003 --artificial-error-rate 0.2 > /tmp/worker3.log 2>&1 &
WORKER3_PID=$!

echo ""
echo "⏳ Waiting 3 seconds for workers to be ready..."
sleep 3

# Start load balancer in background
echo ""
echo "🚀 Starting load balancer with adaptive mode..."
echo "   (Logs being written to /tmp/loadbalancer.log)"
cargo run --bin load_balancer --quiet -- --config config.adaptive.toml > /tmp/loadbalancer.log 2>&1 &
LB_PID=$!

echo ""
echo "⏳ Waiting 5 seconds for load balancer to be ready..."
sleep 5

# Verify load balancer is running
if ! curl -s http://localhost:8080/admin/strategy > /dev/null 2>&1; then
    echo "❌ Load balancer failed to start! Check /tmp/loadbalancer.log"
    tail /tmp/loadbalancer.log
    exit 1
fi

echo "✅ Load balancer is ready!"

# Trap Ctrl+C to kill everything
trap "echo ''; echo '🛑 Stopping all processes...'; kill $LB_PID $WORKER1_PID $WORKER2_PID $WORKER3_PID $TRAFFIC_PID $MONITOR_PID 2>/dev/null; pkill -9 -f 'worker --port' 2>/dev/null; pkill -9 -f 'load_balancer --config' 2>/dev/null; echo '✅ Cleanup complete'; exit 0" INT

echo ""
echo "📊 Initial strategy:"
curl -s http://localhost:8080/admin/strategy | jq -r '.current_strategy' | sed 's/^/   /'
echo ""

echo "🔥 Generating traffic to trigger adaptive behavior..."
echo "   Sending 100 requests with 0.3s interval (will take ~30 seconds)"
echo ""

# Monitor strategy changes in background
(
    LAST_STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy')
    while true; do
        sleep 3
        CURRENT_STRATEGY=$(curl -s http://localhost:8080/admin/strategy 2>/dev/null | jq -r '.current_strategy')
        if [ "$CURRENT_STRATEGY" != "$LAST_STRATEGY" ] && [ ! -z "$CURRENT_STRATEGY" ]; then
            echo ""
            echo "🔄 STRATEGY CHANGED: $LAST_STRATEGY → $CURRENT_STRATEGY"
            echo ""
            LAST_STRATEGY=$CURRENT_STRATEGY
        fi
    done
) &
MONITOR_PID=$!

# Generate traffic in background
(
    for i in {1..100}; do
        RESPONSE=$(curl -s -w "\n%{http_code}" http://localhost:8080/ 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        if [ "$HTTP_CODE" == "200" ]; then
            echo -n "."
        else
            echo -n "E"
        fi
        sleep 0.3
    done
    echo ""
    echo "✅ Traffic generation complete (100 requests sent)"
) &
TRAFFIC_PID=$!

echo "Legend: . = success, E = error"
echo ""

# Wait for traffic to finish
wait $TRAFFIC_PID

echo ""
echo "⏳ Waiting 15 more seconds for adaptive engine to evaluate..."
sleep 15

echo ""
echo "📊 Final strategy:"
curl -s http://localhost:8080/admin/strategy | jq -r '.current_strategy' | sed 's/^/   /'

echo ""
echo "📈 Performance summary from load balancer logs:"
grep -E "(Strategy switched|High latency|High error)" /tmp/loadbalancer.log | tail -5 || echo "   No strategy switches detected"

echo ""
echo "🏁 Test complete! Press Ctrl+C to stop all processes."
echo ""

# Keep everything running
while true; do
    sleep 1
done
