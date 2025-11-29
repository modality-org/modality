#!/usr/bin/env bash
# Recover a partitioned node by restarting it
# This demonstrates network healing and catch-up synchronization

cd $(dirname -- "$0")
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <node_number>"
    echo "Example: $0 4"
    echo "Supported nodes: 1, 2, 3, 4"
    exit 1
fi

NODE_NUM=$1

echo "=== Recovering node${NODE_NUM} ==="
echo "Restarting node${NODE_NUM}..."

# Don't clear storage to allow catch-up from existing state
case $NODE_NUM in
    1)
        modal node run-validator --dir ./tmp/node1 &
        ;;
    2)
        modal node run-validator --dir ./tmp/node2 &
        ;;
    3)
        modal node run-validator --dir ./tmp/node3 &
        ;;
    4)
        modal node run-validator --dir ./tmp/node4 &
        ;;
    *)
        echo "Invalid node number: $NODE_NUM"
        exit 1
        ;;
esac

NODE_PID=$!
echo "Node${NODE_NUM} restarted with PID: $NODE_PID"
echo "Node will sync with network and rejoin consensus..."
echo "$NODE_PID"

