#!/usr/bin/env bash
# Push local commits to sequencers on the network
# Requires a running sequencer node

set -e
cd "$(dirname "$0")"

CONTRACT_DIR="./tmp/my-contract"
NODE_DIR="./tmp/sequencer-node"

echo "📤 Pushing commits to sequencers..."
echo "==================================="
echo ""

# Ensure contract exists
if [ ! -d "$CONTRACT_DIR/.contract" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first."
    exit 1
fi

# Check if node is needed
if [ ! -d "$NODE_DIR" ]; then
    echo "⚙️  Setting up sequencer node..."
    modal node create --dir "$NODE_DIR" --from-template devnet1/node1
    echo "✅ Sequencer node created"
    echo ""
fi

# Start the sequencer node if not running
if ! lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "🚀 Starting sequencer node..."
    cd "$NODE_DIR"
    modal node run-sequencer > ../test-logs/sequencer.log 2>&1 &
    SEQUENCER_PID=$!
    cd - > /dev/null
    
    # Wait for node to be ready
    echo "⏳ Waiting for sequencer to start..."
    for i in {1..30}; do
        if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo "✅ Sequencer ready on port 10101"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo "❌ Timeout waiting for sequencer"
            exit 1
        fi
    done
    sleep 2  # Extra time for full initialization
    echo ""
fi

cd "$CONTRACT_DIR"

REMOTE="/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"

# Push commits
echo "Pushing commits to network..."
modal contract push --remote "$REMOTE" --output json | tee push_result.json
echo ""

# Parse result
if grep -q '"success":true' push_result.json 2>/dev/null || grep -q '"pushed":' push_result.json 2>/dev/null; then
    PUSHED_COUNT=$(cat push_result.json | grep -o '"pushed":[0-9]*' | grep -o '[0-9]*' || echo "?")
    echo "✅ Commits pushed successfully!"
    echo ""
    echo "📊 Push Summary:"
    echo "   Commits pushed: $PUSHED_COUNT"
else
    echo "⚠️  Push completed (check output for details)"
fi

echo ""
echo "💡 Tip: Verify with: modal contract status"
echo "💡 Tip: Pull from network with: modal contract pull"

