#!/usr/bin/env bash
# Integration test for 02-run-devnet2 example
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
test_init "02-run-devnet2"

# Test 1: Create node1 with devnet2/node1 template
echo ""
echo "Test 1: Creating node1 with template..."
assert_success "modal node create --dir ./tmp/node1 --from-template devnet2/node1" "Should create node1 from template"

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
assert_success "[ '$NODE1_PEER_ID' = '12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd' ]" "Should have standard devnet2/node1 peer ID"

# Test 4: Verify config uses port 10201
echo ""
echo "Test 4: Verifying node1 uses port 10201..."
if grep -q "10201" ./tmp/node1/config.json; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Config should use port 10201"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Config should use port 10201"
fi

# Test 5: Create node2 with devnet2/node2 template
echo ""
echo "Test 5: Creating node2 with template..."
assert_success "modal node create --dir ./tmp/node2 --from-template devnet2/node2" "Should create node2 from template"

# Test 6: Verify node2 was created with correct files
echo ""
echo "Test 6: Verifying node2 structure..."
assert_file_exists "./tmp/node2/config.json" "Node2 config.json should exist"
assert_file_exists "./tmp/node2/node.passfile" "Node2 passfile should exist"
assert_file_exists "./tmp/node2/storage" "Node2 storage directory should exist"

# Test 7: Verify node2 has the standard peer ID
echo ""
echo "Test 7: Verifying node2 has standard peer ID..."
NODE2_PEER_ID=$(modal node info --dir ./tmp/node2 2>&1 | grep "Peer ID" | head -1 | awk '{print $3}')
echo "Node2 Peer ID: $NODE2_PEER_ID" >> "$CURRENT_LOG"
assert_success "[ '$NODE2_PEER_ID' = '12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB' ]" "Should have standard devnet2/node2 peer ID"

# Test 8: Verify config uses port 10202
echo ""
echo "Test 8: Verifying node2 uses port 10202..."
if grep -q "10202" ./tmp/node2/config.json; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Config should use port 10202"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Config should use port 10202"
fi

# Test 9: Start node1
echo ""
echo "Test 9: Starting node1..."
NODE1_PID=$(test_start_process "cd ./tmp/node1 && PATH=../../../../../rust/target/debug:\$PATH modal node run --enable-consensus" "node1")

# Wait for node1 to be ready on port 10201
assert_success "test_wait_for_port 10201" "Node1 should start on port 10201"
sleep 2  # Give it a moment to fully initialize

# Test 10: Start node2
echo ""
echo "Test 10: Starting node2..."
NODE2_PID=$(test_start_process "cd ./tmp/node2 && PATH=../../../../../rust/target/debug:\$PATH modal node run --enable-consensus" "node2")

# Wait for node2 to be ready on port 10202
assert_success "test_wait_for_port 10202" "Node2 should start on port 10202"
sleep 2  # Give it a moment to fully initialize

# Test 11: Verify both nodes are still running
echo ""
echo "Test 11: Verifying both nodes are running..."
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"
assert_success "kill -0 $NODE2_PID" "Node2 should still be running"

# Finalize test
test_finalize
exit $?

