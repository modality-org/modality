#!/usr/bin/env bash
set -e

echo "Starting test node..."

# Start the node in background
modal node run \
    --config ./tmp/test-network-params/config.json \
    > ./tmp/test-network-params/node.log 2>&1 &

NODE_PID=$!
echo "$NODE_PID" > ./tmp/test-network-params/node.pid

echo "✓ Node started with PID: $NODE_PID"
echo "  Log file: ./tmp/test-network-params/node.log"

# Wait for node to initialize
echo "Waiting for node to initialize..."
sleep 5

# Check if node is still running
if ! kill -0 $NODE_PID 2>/dev/null; then
    echo "Error: Node process died"
    echo "Last 50 lines of log:"
    tail -50 ./tmp/test-network-params/node.log
    exit 1
fi

echo "✓ Node is running"

