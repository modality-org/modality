#!/usr/bin/env bash
# Push local commits to validators on the network
# Requires a running validator node

set -e
cd "$(dirname "$0")"

CONTRACT_DIR="./tmp/my-contract"
NODE_DIR="./tmp/validator-node"

echo "📤 Pushing commits to validators..."
echo "==================================="
echo ""

# Ensure contract exists
if [ ! -d "$CONTRACT_DIR/.contract" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first."
    exit 1
fi

# Check if node is needed
if [ ! -d "$NODE_DIR" ]; then
    echo "⚙️  Setting up validator node..."
    modal node create --dir "$NODE_DIR" --from-template devnet1/node1
    echo "✅ Validator node created"
    echo ""
fi

# Start the validator node if not running
if ! lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "🚀 Starting validator node..."
    cd "$NODE_DIR"
    modal node run-validator > ../test-logs/validator.log 2>&1 &
    VALIDATOR_PID=$!
    cd - > /dev/null
    
    # Wait for node to be ready
    echo "⏳ Waiting for validator to start..."
    for i in {1..30}; do
        if lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo "✅ Validator ready on port 10101"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo "❌ Timeout waiting for validator"
            exit 1
        fi
    done
    sleep 2  # Extra time for full initialization
    echo ""
fi

cd "$CONTRACT_DIR"

# Push commits
echo "Pushing commits to network..."
modal contract push --output json | tee push_result.json
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

