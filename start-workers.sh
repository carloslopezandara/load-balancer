#!/bin/bash
# start-workers.sh
# Reads TOML configuration and starts workers with their configured artificial behavior
# Usage: ./start-workers.sh [config-file]
#   Default: config.adaptive.toml

set -e

CONFIG_FILE="${1:-config.adaptive.toml}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting Workers from Configuration${NC}"
echo "========================================"
echo ""

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}❌ Configuration file not found: $CONFIG_FILE${NC}"
    exit 1
fi

echo -e "📄 Reading configuration from: ${GREEN}$CONFIG_FILE${NC}"
echo ""
sleep 5

# Check if toml-cli is available, if not use grep/sed parsing
if ! command -v toml &> /dev/null; then
    echo -e "${YELLOW}ℹ️  Using basic TOML parsing (install 'toml-cli' for better parsing)${NC}"
    echo ""
    
    # Extract worker configurations using grep and sed
    # This is a simple parser that works for our specific TOML structure
    
    WORKER_INDEX=0
    CURRENT_URL=""
    CURRENT_DELAY=""
    CURRENT_ERROR=""
    IN_WORKER_BLOCK=false
    
    while IFS= read -r line; do
        # Detect start of worker block
        if [[ "$line" =~ ^\[\[workers\.hosts\]\] ]]; then
            # If we have a previous worker, start it
            if [ ! -z "$CURRENT_URL" ]; then
                # Extract port from URL
                PORT=$(echo "$CURRENT_URL" | grep -oP '(?<=:)\d+$')
                
                if [ ! -z "$PORT" ]; then
                    echo -e "${GREEN}Starting Worker $((WORKER_INDEX + 1)):${NC}"
                    echo "  URL: $CURRENT_URL"
                    echo "  Port: $PORT"
                    echo "  Artificial Delay: ${CURRENT_DELAY}ms"
                    echo "  Artificial Error Rate: ${CURRENT_ERROR}"
                    
                    # Build command
                    CMD="cargo run --bin worker --quiet -- --port $PORT"
                    
                    if [ "$CURRENT_DELAY" != "0" ]; then
                        CMD="$CMD --artificial-delay-ms $CURRENT_DELAY"
                    fi
                    
                    if [ "$CURRENT_ERROR" != "0.0" ] && [ "$CURRENT_ERROR" != "0" ]; then
                        CMD="$CMD --artificial-error-rate $CURRENT_ERROR"
                    fi
                    
                    # Start worker in background
                    LOG_FILE="/tmp/worker${PORT}.log"
                    eval "$CMD" > "$LOG_FILE" 2>&1 &
                    WORKER_PID=$!
                    
                    echo -e "  ${GREEN}✓${NC} Started with PID: $WORKER_PID (logs: $LOG_FILE)"
                    echo ""
                    
                    # Give it a moment to start
                    sleep 1
                    
                    WORKER_INDEX=$((WORKER_INDEX + 1))
                fi
            fi
            
            # Reset for next worker
            CURRENT_URL=""
            CURRENT_DELAY="0"
            CURRENT_ERROR="0.0"
            IN_WORKER_BLOCK=true
            
        elif [ "$IN_WORKER_BLOCK" = true ]; then
            # Extract URL
            if [[ "$line" =~ ^url\ *=\ *\"([^\"]+)\" ]]; then
                CURRENT_URL="${BASH_REMATCH[1]}"
            fi
            
            # Extract enabled status
            if [[ "$line" =~ ^enabled\ *=\ *(true|false) ]]; then
                ENABLED="${BASH_REMATCH[1]}"
                if [ "$ENABLED" = "false" ]; then
                    # Skip disabled workers
                    CURRENT_URL=""
                fi
            fi
            
            # Extract artificial_delay_ms
            if [[ "$line" =~ ^artificial_delay_ms\ *=\ *([0-9]+) ]]; then
                CURRENT_DELAY="${BASH_REMATCH[1]}"
            fi
            
            # Extract artificial_error_rate
            if [[ "$line" =~ ^artificial_error_rate\ *=\ *([0-9.]+) ]]; then
                CURRENT_ERROR="${BASH_REMATCH[1]}"
            fi
            
            # Detect end of worker block (next section or another worker)
            if [[ "$line" =~ ^\[ ]] && [[ ! "$line" =~ ^\[\[workers\.hosts\]\] ]]; then
                IN_WORKER_BLOCK=false
            fi
        fi
    done < "$CONFIG_FILE"
    
    # Don't forget the last worker
    if [ ! -z "$CURRENT_URL" ]; then
        PORT=$(echo "$CURRENT_URL" | grep -oP '(?<=:)\d+$')
        
        if [ ! -z "$PORT" ]; then
            echo -e "${GREEN}Starting Worker $((WORKER_INDEX + 1)):${NC}"
            echo "  URL: $CURRENT_URL"
            echo "  Port: $PORT"
            echo "  Artificial Delay: ${CURRENT_DELAY}ms"
            echo "  Artificial Error Rate: ${CURRENT_ERROR}"
            
            CMD="cargo run --bin worker --quiet -- --port $PORT"
            
            if [ "$CURRENT_DELAY" != "0" ]; then
                CMD="$CMD --artificial-delay-ms $CURRENT_DELAY"
            fi
            
            if [ "$CURRENT_ERROR" != "0.0" ] && [ "$CURRENT_ERROR" != "0" ]; then
                CMD="$CMD --artificial-error-rate $CURRENT_ERROR"
            fi
            
            LOG_FILE="/tmp/worker${PORT}.log"
            eval "$CMD" > "$LOG_FILE" 2>&1 &
            WORKER_PID=$!
            
            echo -e "  ${GREEN}✓${NC} Started with PID: $WORKER_PID (logs: $LOG_FILE)"
            echo ""
            
            sleep 1
            
            WORKER_INDEX=$((WORKER_INDEX + 1))
        fi
    fi
    
    echo "========================================"
    echo -e "${GREEN}✅ Started $WORKER_INDEX worker(s)${NC}"
    echo ""
    echo "Logs are being written to /tmp/workerXXXX.log"
    echo ""
    echo "To stop all workers, run:"
    echo "  pkill -f 'worker --port'"
    echo ""
    
else
    echo -e "${GREEN}✓${NC} Using toml-cli for parsing"
    echo ""
    
    # Use toml-cli for better parsing
    # TODO: Implement toml-cli parsing if user has it installed
    echo -e "${YELLOW}toml-cli parsing not implemented yet, falling back to grep/sed${NC}"
    exit 1
fi
