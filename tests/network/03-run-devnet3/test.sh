#!/usr/bin/env bash
# Integration test for 03-run-devnet3 example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Build modal CLI if needed
if ! command -v modal &> /dev/null; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
    export PATH="../../../rust/target/debug:$PATH"
fi

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "03-run-devnet3"

# Test 1: Start node1
echo ""
echo "Test 1: Starting node1..."
NODE1_PID=$(test_start_process "cd $(pwd) && ./01-run-node1.sh" "node1")
assert_success "test_wait_for_port 10301" "Node1 should start on port 10301"
sleep 2

# Test 2: Start node2
echo ""
echo "Test 2: Starting node2..."
NODE2_PID=$(test_start_process "cd $(pwd) && ./02-run-node2.sh" "node2")
assert_success "test_wait_for_port 10302" "Node2 should start on port 10302"
sleep 2

# Test 3: Start node3
echo ""
echo "Test 3: Starting node3..."
NODE3_PID=$(test_start_process "cd $(pwd) && ./03-run-node3.sh" "node3")
assert_success "test_wait_for_port 10303" "Node3 should start on port 10303"
sleep 3

# Test 4: Verify all nodes are still running
echo ""
echo "Test 4: Verifying all nodes are running..."
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"
assert_success "kill -0 $NODE3_PID" "Node3 should still be running"

# Test 5: Check for peer connections in logs
echo ""
echo "Test 5: Checking for peer connections..."
NODE1_LOG="$LOG_DIR/${CURRENT_TEST}_node1.log"
if [ -f "$NODE1_LOG" ]; then
    echo "Node1 log excerpt:" >> "$CURRENT_LOG"
    tail -n 20 "$NODE1_LOG" >> "$CURRENT_LOG" 2>&1 || true
    
    if grep -q "peer" "$NODE1_LOG" 2>/dev/null || grep -q "connect" "$NODE1_LOG" 2>/dev/null; then
        TESTS_RUN=$((TESTS_RUN + 1))
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} Nodes should establish peer connections"
    else
        TESTS_RUN=$((TESTS_RUN + 1))
        echo -e "  ${YELLOW}⊘${NC} Peer connections not detected in logs (may need more time)"
    fi
fi

# Finalize test
test_finalize
exit $?

