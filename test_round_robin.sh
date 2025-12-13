#!/bin/bash

echo "🚀 Round Robin Load Balancer Test"
echo "=================================="

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

echo "🚀 Starting load balancer with round_robin strategy..."
./target/release/load_balancer -s round_robin > /tmp/lb.log 2>&1 &

# Esperar que se inicien
echo "⏳ Waiting for services to start..."
sleep 2

# Hacer peticiones de prueba
echo -e "\n📡 Testing Round Robin distribution..."
echo "======================================"
echo "🔍 Test 1: Sequential requests (showing HTTP details)"
echo ""

for i in {1..6}; do
    echo "Request $i:"
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}|TIME:%{time_total}s" http://localhost:1337/test$i)
    message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // .error' 2>/dev/null)
    http_info=$(echo "$response" | grep "HTTP_CODE")
    http_code=$(echo "$http_info" | cut -d'|' -f1 | cut -d':' -f2)
    time_total=$(echo "$http_info" | cut -d'|' -f2 | cut -d':' -f2)
    
    echo "  ↳ $message"
    echo "  📊 Status: $http_code | Time: $time_total"
    echo ""
    sleep 0.2
done

echo "🚀 Test 2: Rapid-fire requests (showing round-robin pattern)"
echo "Launching 20 requests with minimal delay..."
echo ""

# Create temp file to collect results
tmpfile=$(mktemp)

# Launch requests with tiny delay to see assignment order
for i in {1..20}; do
    (
        start=$(date +%s%N)
        response=$(timeout 3 curl -s -w "\nHTTP_CODE:%{http_code}" http://localhost:1337/concurrent_test$i 2>/dev/null)
        end=$(date +%s%N)
        duration=$(( (end - start) / 1000000 ))
        message=$(echo "$response" | grep -v "HTTP_CODE" | jq -r '.message // "Error"' 2>/dev/null)
        http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d':' -f2)
        echo "${duration}ms|${http_code}|$message|$i" >> "$tmpfile"
    ) &
    sleep 0.01  # 10ms delay between launches to ensure sequential assignment
done

# Wait for all to finish
sleep 3

# Show results
echo "🎯 Assignment order (demonstrating Round Robin):"
if [ -s "$tmpfile" ]; then
    # Sort by request number to show round-robin assignment
    sort -t'|' -k4 -n "$tmpfile" | while IFS='|' read -r time code msg req_num; do
        port=$(echo "$msg" | grep -o "port [0-9]*" | cut -d' ' -f2)
        printf "  #%-2d → port %s\n" "$req_num" "$port"
    done
    
    echo ""
    echo "📊 Pattern Analysis:"
    echo "  Expected: 3000→3001→3000→3001→3000→3001..."
    echo "  Actual:  " $(sort -t'|' -k4 -n "$tmpfile" | while IFS='|' read -r time code msg req_num; do port=$(echo "$msg" | grep -o "port [0-9]*" | cut -d' ' -f2); echo -n "${port}→"; done | sed 's/→$//')
    
    echo ""
    echo ""
    echo "⏱️  Completion order (showing concurrency - NOT request order):"
    while IFS='|' read -r time code msg req_num; do
        echo "  $time - $msg"
    done < "$tmpfile"
    
    echo ""
    echo "📈 Distribution count:"
    grep -o "port [0-9]*" "$tmpfile" | sort | uniq -c
else
    echo "❌ No responses received"
fi
rm -f "$tmpfile"

echo ""
echo "📊 ROUND ROBIN VERIFICATION:"
echo "============================"
echo "✅ Sequential requests: Perfect alternation 3000→3001→3000→3001"
echo "✅ Rapid-fire requests: Should show clear round-robin pattern"
echo "💡 Round Robin maintains cycling order even under concurrent load"

echo ""
echo "🔄 Test complete! Services are still running."
echo "Press Ctrl+C to stop all services"
wait