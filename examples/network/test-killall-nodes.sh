#!/bin/bash
# Test script for modal local killall-nodes command

set -e

cd "$(dirname "$0")/../.."
MODAL_BIN="./rust/target/debug/modal"

echo "Building modal CLI..."
cd rust && cargo build --package modal && cd ..

echo ""
echo "=== Testing modal local killall-nodes command ==="
echo ""

# Clean up any existing test nodes
rm -rf /tmp/test-killall

# Start a few test nodes
echo "Starting 3 test validator nodes..."
mkdir -p /tmp/test-killall/node1 /tmp/test-killall/node2 /tmp/test-killall/node3

# Create node1
cd /tmp/test-killall/node1
$MODAL_BIN node create --type validator --port 15001
$MODAL_BIN node run-validator --dir . > node1.log 2>&1 &
echo "Started node1"

# Create node2
cd /tmp/test-killall/node2
$MODAL_BIN node create --type validator --port 15002
$MODAL_BIN node run-validator --dir . > node2.log 2>&1 &
echo "Started node2"

# Create node3
cd /tmp/test-killall/node3
$MODAL_BIN node create --type validator --port 15003
$MODAL_BIN node run-validator --dir . > node3.log 2>&1 &
echo "Started node3"

# Wait for nodes to initialize
sleep 3

echo ""
echo "=== Check running nodes before kill ==="
cd /tmp/test-killall
$MODAL_BIN local nodes

echo ""
echo "=== Test dry-run mode ==="
$MODAL_BIN local killall-nodes --dry-run

echo ""
echo "=== Kill all nodes with graceful shutdown ==="
$MODAL_BIN local killall-nodes

echo ""
echo "=== Check that no nodes are running ==="
sleep 2
$MODAL_BIN local nodes

echo ""
echo "=== Test force kill with running nodes ==="
echo "Starting 2 more nodes..."

cd /tmp/test-killall/node1
$MODAL_BIN node run-validator --dir . > node1.log 2>&1 &

cd /tmp/test-killall/node2
$MODAL_BIN node run-validator --dir . > node2.log 2>&1 &

sleep 3
echo ""
echo "Running nodes:"
$MODAL_BIN local nodes

echo ""
echo "Force killing all nodes..."
$MODAL_BIN local killall-nodes --force

sleep 2
echo ""
echo "Check no nodes running:"
$MODAL_BIN local nodes

# Clean up
echo ""
echo "Cleaning up test directories..."
rm -rf /tmp/test-killall

echo ""
echo "✓ Test complete"

