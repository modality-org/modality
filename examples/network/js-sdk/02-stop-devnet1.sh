#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Check if node1 PID file exists
if [ ! -f "./tmp/node1.pid" ]; then
    echo "No node1 PID file found. Node may not be running."
    exit 0
fi

NODE_PID=$(cat ./tmp/node1.pid)

# Kill the node
if ps -p ${NODE_PID} > /dev/null; then
    echo "Stopping node1 (PID: ${NODE_PID})..."
    kill ${NODE_PID}
    
    # Wait for process to stop
    sleep 2
    
    # Force kill if still running
    if ps -p ${NODE_PID} > /dev/null; then
        echo "Force stopping node1..."
        kill -9 ${NODE_PID}
    fi
    
    echo "✓ Node1 stopped"
else
    echo "Node1 (PID: ${NODE_PID}) is not running"
fi

# Clean up PID file
rm -f ./tmp/node1.pid

echo "✓ Cleanup complete"

