#!/usr/bin/env bash
# Simulate partitioning two nodes (node3 and node4) from the network
# This demonstrates the Byzantine threshold: with f=1, losing 2 nodes means no quorum

cd $(dirname -- "$0")
set -e

if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Usage: $0 <node3_pid> <node4_pid>"
    echo "Example: $0 12345 12346"
    exit 1
fi

NODE3_PID=$1
NODE4_PID=$2

echo "=== Simulating two-node partition ==="
echo "Killing node3 (PID: $NODE3_PID)..."
kill -9 "$NODE3_PID" 2>/dev/null || true

echo "Killing node4 (PID: $NODE4_PID)..."
kill -9 "$NODE4_PID" 2>/dev/null || true

echo ""
echo "Two nodes partitioned. Network now has only 2 validators."
echo "With n=4, f=1, we need quorum of 2f+1 = 3 validators."
echo "Consensus CANNOT continue - this demonstrates Byzantine threshold."

