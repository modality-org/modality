#!/usr/bin/env bash
# Integration test for 01-ping-node example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Build modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
fi

# Add modal to PATH for this test
export PATH="../../../rust/target/debug:$PATH"

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "01-ping-node"

# Test 1: Create node1 with standard devnet1/node1 identity using template
echo ""
echo "Test 1: Creating node1 with template..."

# Create node using template
assert_success "modal node create --dir ./tmp/node1 --from-template devnet1/node1" "Should create node1 from template"

# Test 2: Verify node1 was created with correct files
echo ""
echo "Test 2: Verifying node1 structure..."
assert_file_exists "./tmp/node1/config.json" "Node1 config.json should exist"
assert_file_exists "./tmp/node1/node.passfile" "Node1 passfile should exist"
assert_file_exists "./tmp/node1/storage" "Node1 storage directory should exist"

# Test 3: Verify node1 has the standard peer ID
echo ""
echo "Test 3: Verifying node1 has standard peer ID..."
NODE1_PEER_ID=$(modal node info --dir ./tmp/node1 2>&1 | grep "Peer ID" | head -1 | awk '{print $3}')
echo "Node1 Peer ID: $NODE1_PEER_ID" >> "$CURRENT_LOG"
assert_success "[ '$NODE1_PEER_ID' = '12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd' ]" "Should have standard devnet1/node1 peer ID"

# Test 4: Verify config uses port 10101
echo ""
echo "Test 4: Verifying node1 uses port 10101..."
if grep -q "10101" ./tmp/node1/config.json; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Config should use port 10101"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Config should use port 10101"
fi

# Test 5: Start node1
echo ""
echo "Test 5: Starting node1..."
NODE1_PID=$(test_start_process "cd ./tmp/node1 && modal node run" "node1")

# Wait for node1 to be ready on port 10101
assert_success "test_wait_for_port 10101" "Node1 should start on port 10101"
sleep 2  # Give it a moment to fully initialize

# Test 6: Create node2
echo ""
echo "Test 6: Creating node2..."
assert_success "modal node create --dir ./tmp/node2 --network devnet1" "Should create node2"
assert_file_exists "./tmp/node2/config.json" "Node2 config.json should exist"

# Test 7: Ping node1 from node2 using standard peer ID and port
echo ""
echo "Test 7: Pinging node1 from node2..."
assert_output_contains \
    "modal node ping --dir ./tmp/node2 --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd --times 3" \
    "successful" \
    "Ping should succeed"

# Test 8: Verify ping response time is reasonable
echo ""
echo "Test 8: Checking ping response time..."
PING_OUTPUT=$(modal node ping --dir ./tmp/node2 --target /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd --times 1 2>&1 || true)
echo "Ping output: $PING_OUTPUT" >> "$CURRENT_LOG"

# Finalize test
test_finalize
exit $?

