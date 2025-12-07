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

# Usar puertos originales
echo "🚀 Starting worker on port 3000..."
./target/release/worker 3000 &

echo "🚀 Starting worker on port 3001..."
./target/release/worker 3001 &

echo "🚀 Starting load balancer on port 1337..."
./target/release/load_balancer &

# Esperar que se inicien
echo "⏳ Waiting for services to start..."
sleep 3

# Cambiar a estrategia Round Robin
echo "🔄 Switching to Round Robin strategy..."
curl -s -X POST -H "Content-Type: application/json" -d '{"strategy":"round_robin"}' http://localhost:1337/admin/strategy > /dev/null
sleep 1

# Hacer peticiones de prueba
echo -e "\n📡 Testing Round Robin distribution..."
echo "======================================"

for i in {1..8}; do
    echo "Request $i:"
    curl -s http://localhost:1337/round_robin_test$i
    echo -e "\n"
    sleep 0.5
done

echo "🔄 Round Robin test complete! Services are still running..."
echo "Press Ctrl+C to stop all services"
wait