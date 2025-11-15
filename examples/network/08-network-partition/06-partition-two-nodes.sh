#!/usr/bin/env bash
# Simulate partitioning two nodes (node3 and node4) from the network
# This demonstrates the Byzantine threshold: with f=1, losing 2 nodes means no quorum

cd $(dirname -- "$0")
set -e

echo "=== Simulating two-node partition ==="

# Use modal node kill if available, fall back to PID-based killing
if command -v modal &> /dev/null; then
    if [ -d "./tmp/node3" ]; then
        echo "Killing node3 using modal node kill..."
        modal node kill --dir ./tmp/node3 --force 2>/dev/null || echo "Node3 may not be running"
    fi
    
    if [ -d "./tmp/node4" ]; then
        echo "Killing node4 using modal node kill..."
        modal node kill --dir ./tmp/node4 --force 2>/dev/null || echo "Node4 may not be running"
    fi
else
    # Fallback to PID-based killing
    if [ -z "$1" ] || [ -z "$2" ]; then
        echo "Usage: $0 <node3_pid> <node4_pid>"
        echo "Example: $0 12345 12346"
        exit 1
    fi
    
    NODE3_PID=$1
    NODE4_PID=$2
    
    echo "Killing node3 (PID: $NODE3_PID)..."
    kill -9 "$NODE3_PID" 2>/dev/null || true
    
    echo "Killing node4 (PID: $NODE4_PID)..."
    kill -9 "$NODE4_PID" 2>/dev/null || true
fi

echo ""
echo "Two nodes partitioned. Network now has only 2 validators."
echo "With n=4, f=1, we need quorum of 2f+1 = 3 validators."
echo "Consensus CANNOT continue - this demonstrates Byzantine threshold."

