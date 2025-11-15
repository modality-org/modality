#!/usr/bin/env bash
# Simulate partitioning a single node (node4) from the network
# This demonstrates that consensus can continue with n-1 validators (f=1 tolerable)

cd $(dirname -- "$0")
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <node4_pid>"
    echo "Example: $0 12345"
    exit 1
fi

NODE4_PID=$1

echo "=== Simulating single node partition ==="
echo "Killing node4 (PID: $NODE4_PID)..."
kill -9 "$NODE4_PID" 2>/dev/null || true

echo "Node4 partitioned. Network should continue with 3 validators (quorum = 3)."
echo "Consensus can continue as we have exactly 2f+1 validators remaining."

