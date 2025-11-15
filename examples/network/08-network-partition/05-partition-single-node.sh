#!/usr/bin/env bash
# Simulate partitioning a single node (node4) from the network
# This demonstrates that consensus can continue with n-1 validators (f=1 tolerable)

cd $(dirname -- "$0")
set -e

echo "=== Simulating single node partition ==="
echo "Killing node4 using modal node kill..."

# Use modal node kill if available, fall back to PID-based killing
if command -v modal &> /dev/null && [ -d "./tmp/node4" ]; then
    modal node kill --dir ./tmp/node4 --force 2>/dev/null || echo "Node4 may not be running"
else
    # Fallback to PID-based killing
    if [ -z "$1" ]; then
        echo "Usage: $0 <node4_pid>"
        echo "Example: $0 12345"
        exit 1
    fi
    NODE4_PID=$1
    echo "Killing node4 (PID: $NODE4_PID)..."
    kill -9 "$NODE4_PID" 2>/dev/null || true
fi

echo "Node4 partitioned. Network should continue with 3 validators (quorum = 3)."
echo "Consensus can continue as we have exactly 2f+1 validators remaining."

