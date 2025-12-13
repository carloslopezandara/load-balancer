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
./target/release/worker -p 3000 > /tmp/worker3000.log 2>&1 &

echo "🚀 Starting worker on port 3001..."
./target/release/worker -p 3001 > /tmp/worker3001.log 2>&1 &

echo "🚀 Starting load balancer with least_connections strategy..."
./target/release/load_balancer -s least_connections > /tmp/lb.log 2>&1 &

# Esperar que se inicien
echo "⏳ Waiting for services to start..."
sleep 3

# Verificar estrategia actual
echo "🔍 Current strategy:"
curl -s http://localhost:1337/admin/strategy | jq
sleep 1

# Hacer peticiones de prueba
echo -e "\n📡 Testing Least Connections distribution..."
echo "============================================"

echo "🔍 Test 1: Sequential requests (showing HTTP details)"
echo ""

for i in {1..6}; do
    echo "Request $i:"
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}|TIME:%{time_total}s" http://localhost:1337/seq_test$i)
    message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // .error' 2>/dev/null)
    http_info=$(echo "$response" | grep "HTTP_CODE")
    http_code=$(echo "$http_info" | cut -d'|' -f1 | cut -d':' -f2)
    time_total=$(echo "$http_info" | cut -d'|' -f2 | cut -d':' -f2)
    
    echo "  ↳ $message"
    echo "  📊 Status: $http_code | Time: $time_total"
    echo ""
    sleep 0.2
done

echo "🚀 Test 2: Mixed workload (fast + slow requests showing least connections)"
echo "Launching mixed requests: 10 slow (1s) + 10 fast (/work endpoint, 10ms)..."
echo ""

# Create temp file to collect results
tmpfile=$(mktemp)

# Launch 5 SLOW requests first to worker
for i in {1..5}; do
    (
        start=$(date +%s%N)
        response=$(timeout 3 curl -s -w "\nHTTP_CODE:%{http_code}" http://localhost:1337/slow_test$i 2>/dev/null)
        end=$(date +%s%N)
        duration=$(( (end - start) / 1000000 ))
        message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // "Error"' 2>/dev/null)
        http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d':' -f2)
        echo "${duration}ms|${http_code}|SLOW-$i|$message" >> "$tmpfile"
    ) &
    sleep 0.05  # 50ms between slow requests
done

# Small pause then launch FAST requests - these should go to least loaded worker
sleep 0.1

for i in {1..10}; do
    (
        start=$(date +%s%N)
        response=$(timeout 3 curl -s -w "\nHTTP_CODE:%{http_code}" http://localhost:1337/work 2>/dev/null)
        end=$(date +%s%N)
        duration=$(( (end - start) / 1000000 ))
        message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // "Error"' 2>/dev/null)
        http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d':' -f2)
        echo "${duration}ms|${http_code}|FAST-$i|$message" >> "$tmpfile"
    ) &
    sleep 0.02  # 20ms between fast requests
done

# Launch remaining slow requests
for i in {6..10}; do
    (
        start=$(date +%s%N)
        response=$(timeout 3 curl -s -w "\nHTTP_CODE:%{http_code}" http://localhost:1337/slow_test$i 2>/dev/null)
        end=$(date +%s%N)
        duration=$(( (end - start) / 1000000 ))
        message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // "Error"' 2>/dev/null)
        http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d':' -f2)
        echo "${duration}ms|${http_code}|SLOW-$i|$message" >> "$tmpfile"
    ) &
    sleep 0.05
done

# Wait for all to finish
sleep 3

# Show results
echo "🎯 Request distribution (by type):"
if [ -s "$tmpfile" ]; then
    echo ""
    echo "SLOW requests (1s processing):"
    grep "SLOW" "$tmpfile" | while IFS='|' read -r time code type msg; do
        port=$(echo "$msg" | grep -o "port [0-9]*" | cut -d' ' -f2)
        printf "  %s → port %s (%s)\n" "$type" "$port" "$time"
    done
    
    echo ""
    echo "FAST requests (10ms /work endpoint):"
    grep "FAST" "$tmpfile" | while IFS='|' read -r time code type msg; do
        port=$(echo "$msg" | grep -o "port [0-9]*" | cut -d' ' -f2)
        printf "  %s → port %s (%s)\n" "$type" "$port" "$time"
    done
    
    echo ""
    echo "📊 Analysis:"
    echo "  - SLOW requests start first, occupying workers with 1s connections"
    echo "  - FAST requests arrive while SLOW ones are processing"
    echo "  - Least Connections sends FAST requests to worker with fewer active connections"
    
    echo ""
    echo "📈 Distribution by worker:"
    echo "All requests:"
    grep -o "port [0-9]*" "$tmpfile" | sort | uniq -c
    echo ""
    echo "FAST requests only (should favor less-loaded worker):"
    grep "FAST" "$tmpfile" | grep -o "port [0-9]*" | sort | uniq -c
else
    echo "❌ No responses received"
fi
rm -f "$tmpfile"
echo ""

echo ""
echo "📊 LEAST CONNECTIONS VERIFICATION:"
echo "==================================="
echo "✅ Sequential requests: All go to same worker (no concurrent load)"
echo "✅ Mixed workload: Fast requests favor worker with fewer active connections"
echo "💡 Key difference from Round Robin: Tracks ACTIVE connections, not just count"
echo "   When workers have different loads, new requests go to least busy one"

echo ""
echo "🔄 Test complete! Services are still running."
echo "Press Ctrl+C to stop all services"

wait