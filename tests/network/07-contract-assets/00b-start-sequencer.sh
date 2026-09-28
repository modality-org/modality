#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0.5: Start devnet1 Sequencer"
echo "================================================"
echo ""

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating sequencer node from devnet1/node1 template..."
    modal node create \
        --dir "./tmp/node1" \
        --from-template devnet1/node1
    echo "✅ Node created"
    echo ""
fi

# Clear storage for fresh start
echo "Clearing sequencer storage..."
modal node clear-storage --dir ./tmp/node1 --yes

# Check if sequencer is already running
if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "⚠️  Sequencer already running on port 10101"
    echo ""
else
    echo "🚀 Starting sequencer node in background..."
    cd tmp/node1
    modal node run-sequencer > ../test-logs/sequencer.log 2>&1 &
    SEQUENCER_PID=$!
    cd ../..
    
    # Save PID for cleanup
    echo $SEQUENCER_PID > tmp/sequencer.pid
    
    echo "⏳ Waiting for sequencer to start..."
    for i in {1..30}; do
        if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo "✅ Sequencer ready on port 10101 (PID: $SEQUENCER_PID)"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo "❌ Timeout waiting for sequencer"
            exit 1
        fi
    done
    sleep 2  # Extra time for initialization
    echo ""
fi

echo "Sequencer is running!"
echo "  - Port: 10101"
echo "  - Peer ID: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
echo "  - Logs: tmp/test-logs/sequencer.log"
echo ""

