#!/bin/bash
# Test script for modal local nodes command

set -e

cd "$(dirname "$0")/../.."
MODAL_BIN="./rust/target/debug/modal"

echo "Building modal CLI..."
cd rust && cargo build --package modal && cd ..

echo ""
echo "=== Testing modal local nodes command ==="
echo ""

# Clean up any existing test nodes
rm -rf /tmp/test-nodes-cmd

# Start a few test nodes
echo "Starting 3 test validator nodes..."
mkdir -p /tmp/test-nodes-cmd/node1 /tmp/test-nodes-cmd/node2 /tmp/test-nodes-cmd/node3

# Create node1
cd /tmp/test-nodes-cmd/node1
$MODAL_BIN node create --type validator --port 14001
$MODAL_BIN node run-validator --dir . > node1.log 2>&1 &
NODE1_PID=$!
echo "Started node1 (PID: $NODE1_PID)"

# Create node2
cd /tmp/test-nodes-cmd/node2
$MODAL_BIN node create --type validator --port 14002
$MODAL_BIN node run-validator --dir . > node2.log 2>&1 &
NODE2_PID=$!
echo "Started node2 (PID: $NODE2_PID)"

# Create node3
cd /tmp/test-nodes-cmd/node3
$MODAL_BIN node create --type validator --port 14003
$MODAL_BIN node run-validator --dir . > node3.log 2>&1 &
NODE3_PID=$!
echo "Started node3 (PID: $NODE3_PID)"

# Wait for nodes to initialize
sleep 3

echo ""
echo "=== Running 'modal local nodes' command ==="
echo ""
cd /tmp/test-nodes-cmd
$MODAL_BIN local nodes

echo ""
echo "=== Running 'modal local nodes --verbose' command ==="
echo ""
$MODAL_BIN local nodes --verbose

# Clean up
echo ""
echo "Cleaning up test nodes..."
kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true
wait 2>/dev/null || true
rm -rf /tmp/test-nodes-cmd

echo ""
echo "✓ Test complete"

