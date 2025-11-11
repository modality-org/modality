#!/usr/bin/env bash
# Integration test for 02-run-devnet1 example
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
test_init "02-run-devnet1"

# Test 1: Create node1 with devnet1/node1 template
echo ""
echo "Test 1: Creating node1 with template..."
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

# Test 5: Start node1 as validator
echo ""
echo "Test 5: Starting node1 as validator..."
NODE1_PID=$(test_start_process "cd ./tmp/node1 && PATH=../../../../../rust/target/debug:\$PATH modal node run-validator" "node1")

# Wait for node1 to be ready on port 10101
assert_success "test_wait_for_port 10101" "Node1 should start on port 10101"
sleep 2  # Give it a moment to fully initialize

# Test 6: Verify node1 is still running
echo ""
echo "Test 6: Verifying node1 is running..."
assert_success "kill -0 $NODE1_PID" "Node1 should still be running"

# Test 7: Verify node is a static validator
echo ""
echo "Test 7: Verifying node1 is running as a static validator..."
sleep 3  # Wait for validator to start
if grep -q "Found 1 static validators" ./tmp/test-logs/02-run-devnet1_node1.log && \
   grep -q "Starting validator node" ./tmp/test-logs/02-run-devnet1_node1.log; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Node1 should be running as a static validator"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Node1 should be running as a static validator"
fi

# Test 8: Create a contract
echo ""
echo "Test 8: Creating a contract..."
CONTRACT_OUTPUT=$(PATH=../../../rust/target/debug:$PATH modal contract create --output json 2>&1)
if [ $? -eq 0 ]; then
    CONTRACT_ID=$(echo "$CONTRACT_OUTPUT" | grep '"contract_id"' | head -1 | sed 's/.*: "\(.*\)".*/\1/')
    echo "Contract ID: $CONTRACT_ID" >> "$CURRENT_LOG"
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Contract created successfully"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Failed to create contract"
    echo "Error: $CONTRACT_OUTPUT" >> "$CURRENT_LOG"
    CONTRACT_ID=""
fi

# Test 9: Submit a commit to the contract (direct storage)
echo ""
echo "Test 9: Submitting a commit to the contract (stopping node first)..."
if [ -n "$CONTRACT_ID" ]; then
    echo "DEBUG: CONTRACT_ID=$CONTRACT_ID" >> "$CURRENT_LOG"
    # Stop node to access storage
    echo "DEBUG: Killing PID $NODE1_PID" >> "$CURRENT_LOG"
    kill -9 $NODE1_PID 2>/dev/null || true
    sleep 3  # Give RocksDB time to release locks
    echo "DEBUG: About to run commit command" >> "$CURRENT_LOG"
    
    COMMIT_OUTPUT=$(timeout 10 bash -c 'PATH=../../../rust/target/debug:$PATH modal contract commit --contract-id "'$CONTRACT_ID'" --path "/test.txt" --value "hello world" --dir ./tmp/node1 --output json' 2>&1)
    if [ $? -eq 0 ]; then
        COMMIT_ID=$(echo "$COMMIT_OUTPUT" | grep '"commit_id"' | sed 's/.*: "\(.*\)".*/\1/')
        echo "Commit ID: $COMMIT_ID" >> "$CURRENT_LOG"
        TESTS_RUN=$((TESTS_RUN + 1))
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} Commit submitted successfully"
        
        # Wait a moment for the commit to be stored
        sleep 1
    else
        TESTS_RUN=$((TESTS_RUN + 1))
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} Failed to submit commit"
        echo "Error: $COMMIT_OUTPUT" >> "$CURRENT_LOG"
        COMMIT_ID=""
    fi
else
    echo -e "  ${YELLOW}⊘${NC} Skipping (no contract ID)"
fi

# Test 10: Retrieve the commit from storage
echo ""
echo "Test 10: Retrieving commit from storage..."
if [ -n "$CONTRACT_ID" ] && [ -n "$COMMIT_ID" ]; then
    GET_OUTPUT=$(PATH=../../../rust/target/debug:$PATH modal contract get --contract-id "$CONTRACT_ID" --commit-id "$COMMIT_ID" --dir ./tmp/node1 --output json 2>&1)
    if [ $? -eq 0 ]; then
        echo "Retrieved commit: $GET_OUTPUT" >> "$CURRENT_LOG"
        TESTS_RUN=$((TESTS_RUN + 1))
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} Commit retrieved successfully"
    else
        TESTS_RUN=$((TESTS_RUN + 1))
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} Failed to retrieve commit"
        echo "Error: $GET_OUTPUT" >> "$CURRENT_LOG"
    fi
else
    echo -e "  ${YELLOW}⊘${NC} Skipping (no contract or commit ID)"
fi

# Finalize test
test_finalize
exit $?

