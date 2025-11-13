#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0.5: Start devnet1 Validator"
echo "================================================"
echo ""

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating validator node from devnet1/node1 template..."
    modal node create \
        --dir "./tmp/node1" \
        --from-template devnet1/node1
    echo "✅ Node created"
    echo ""
fi

# Clear storage for fresh start
echo "Clearing validator storage..."
modal node clear-storage --dir ./tmp/node1 --yes

# Check if validator is already running
if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "⚠️  Validator already running on port 10101"
    echo ""
else
    echo "🚀 Starting validator node in background..."
    cd tmp/node1
    modal node run-validator > ../test-logs/validator.log 2>&1 &
    VALIDATOR_PID=$!
    cd ../..
    
    # Save PID for cleanup
    echo $VALIDATOR_PID > tmp/validator.pid
    
    echo "⏳ Waiting for validator to start..."
    for i in {1..30}; do
        if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo "✅ Validator ready on port 10101 (PID: $VALIDATOR_PID)"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo "❌ Timeout waiting for validator"
            exit 1
        fi
    done
    sleep 2  # Extra time for initialization
    echo ""
fi

echo "Validator is running!"
echo "  - Port: 10101"
echo "  - Peer ID: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
echo "  - Logs: tmp/test-logs/validator.log"
echo ""

