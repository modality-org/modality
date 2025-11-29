#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Try using modal node kill first if the node directory exists
if [ -d "./tmp/node1" ] && command -v modal &> /dev/null; then
    echo "Stopping node1 using modal node kill..."
    modal node kill --dir ./tmp/node1 2>/dev/null && echo "✓ Node1 stopped" && rm -f ./tmp/node1.pid && echo "✓ Cleanup complete" && exit 0 || echo "Trying alternative methods..."
fi

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

