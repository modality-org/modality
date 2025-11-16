#!/usr/bin/env bash
set -e

echo "Stopping test node..."

if [ -f ./tmp/test-network-params/node.pid ]; then
    NODE_PID=$(cat ./tmp/test-network-params/node.pid)
    
    if kill -0 $NODE_PID 2>/dev/null; then
        kill $NODE_PID
        echo "✓ Node stopped (PID: $NODE_PID)"
    else
        echo "✓ Node already stopped"
    fi
else
    echo "✓ No PID file found"
fi

