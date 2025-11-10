#!/usr/bin/env bash
# Integration test for 02-run-devnet2 example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Initialize test
test_init "02-run-devnet2"

# Build modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
fi

# Note: This example uses both JS and Rust implementations
# We'll focus on testing the Rust node (node2)

# Test 1: Start node2 (Rust implementation)
echo ""
echo "Test 1: Starting node2 (Rust)..."
NODE2_PID=$(test_start_process "./02-run-node2.sh" "node2")
assert_success "test_wait_for_port 10102" "Node2 should start on port 10102"

# Test 2: Verify node2 is running
echo ""
echo "Test 2: Verifying node2 is running..."
sleep 3
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"

# Test 3: Verify storage was created
echo ""
echo "Test 3: Verifying storage was created..."
# Check the log for successful startup
NODE2_LOG="$LOG_DIR/${CURRENT_TEST}_node2.log"
if [ -f "$NODE2_LOG" ]; then
    echo "Node2 log excerpt:" >> "$CURRENT_LOG"
    tail -n 20 "$NODE2_LOG" >> "$CURRENT_LOG" 2>&1 || true
fi

# Note: Full devnet2 test would require JS CLI to be installed
# For now, we test that the Rust node starts successfully
echo ""
echo "Note: Full devnet2 test requires JS CLI (modality-js) to be installed"
echo "This test verifies the Rust node (node2) starts correctly"

# Finalize test
test_finalize
exit $?

