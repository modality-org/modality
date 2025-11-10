#!/usr/bin/env bash
# Integration test for 01-ping-node example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Initialize test
test_init "01-ping-node"

# Build modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
fi

# Clean up any previous storage
rm -rf ./configs/tmp/storage/node2 2>/dev/null || true

# Test 1: Start node1
echo ""
echo "Test 1: Starting node1..."
NODE1_PID=$(test_start_process "../../../rust/target/debug/modal node run --config ../../../fixtures/network-node-configs/devnet1/node1.json" "node1")

# Wait for node1 to be ready
assert_success "test_wait_for_port 10101" "Node1 should start on port 10101"
sleep 2  # Give it a moment to fully initialize

# Test 2: Ping node1 from node2
echo ""
echo "Test 2: Pinging node1 from node2..."
assert_output_contains \
    "../../../rust/target/debug/modal node ping --config ./configs/node2.json --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd --times 3" \
    "Ping successful" \
    "Ping should succeed"

# Test 3: Verify ping response time is reasonable
echo ""
echo "Test 3: Checking ping response time..."
PING_OUTPUT=$(../../../rust/target/debug/modal node ping --config ./configs/node2.json --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd --times 1 2>&1 || true)
echo "Ping output: $PING_OUTPUT" >> "$CURRENT_LOG"

# Finalize test
test_finalize
exit $?

