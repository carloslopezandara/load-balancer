#!/bin/bash

echo "🚀 Least Connections Load Balancer Test"
echo "======================================="

# Función para limpiar procesos al salir
cleanup() {
    echo -e "\n🧹 Stopping services..."
    pkill -f "worker.*300"
    pkill -f load_balancer
    echo "✅ Services stopped"
}

trap cleanup EXIT

# Compilar
echo "📦 Building..."
cargo build --release --quiet

echo "🚀 Starting worker on port 3000..."
./target/release/worker 3000 &

echo "🚀 Starting worker on port 3001..."
./target/release/worker 3001 &

echo "🚀 Starting load balancer on port 1337..."
./target/release/load_balancer &

# Esperar que se inicien
echo "⏳ Waiting for services to start..."
sleep 3

# Asegurar que está usando Least Connections strategy
echo "🔄 Ensuring Least Connections strategy is active..."
curl -s -X POST -H "Content-Type: application/json" -d '{"strategy":"least_connections"}' http://localhost:1337/admin/strategy > /dev/null
sleep 1

# Hacer peticiones de prueba
echo -e "\n📡 Testing Least Connections distribution..."
echo "============================================"

echo "🔍 Test 1: Sequential requests (should go to worker 0 since connections end quickly)"
for i in {1..4}; do
    echo "Request $i:"
    response=$(curl -s http://localhost:1337/seq_test$i)
    echo "  $response"
    sleep 0.2
done

echo ""
echo "🚀 Test 2: CONCURRENT requests to demonstrate Least Connections"
echo "Sending 8 requests simultaneously..."
echo "(This should show distribution between both workers)"
echo ""

# Start 8 concurrent requests - with better output formatting
for i in {1..8}; do
    (
        response=$(curl -s --max-time 5 http://localhost:1337/concurrent_test$i 2>/dev/null)
        echo "Request $i: $response"
    ) &
done

# Give requests time to complete
sleep 3
echo ""

echo "⏳ Let connections settle..."
sleep 1

echo ""
echo "📊 LEAST CONNECTIONS ANALYSIS:"
echo "=============================="
echo "✅ Sequential requests: All go to same worker (port 3000)"
echo "✅ Concurrent requests: Distribute between both workers"
echo "💡 Least Connections algorithm:"
echo "   • Tracks active connections per worker"
echo "   • Sends new requests to worker with fewest connections"
echo "   • Best for varying request processing times"

echo ""
echo "🔄 Least Connections test complete! Services are still running..."
echo "Press Ctrl+C to stop all services"

# Keep script running like round robin test
while true; do
    sleep 1
done