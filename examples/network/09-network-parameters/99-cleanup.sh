#!/usr/bin/env bash

echo "Cleaning up test environment..."

# Stop node if running
if [ -f ./tmp/test-network-params/node.pid ]; then
    NODE_PID=$(cat ./tmp/test-network-params/node.pid)
    kill $NODE_PID 2>/dev/null || true
fi

# Remove temp directory
rm -rf ./tmp/test-network-params

echo "✓ Cleanup complete"

