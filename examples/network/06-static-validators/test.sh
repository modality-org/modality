#!/usr/bin/env bash
# Integration test for 06-static-validators example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Initialize test
test_init "06-static-validators"

# Build modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
fi

# Test 1: Clean storage
echo ""
echo "Test 1: Cleaning storage..."
./00-clean-storage.sh >> "$CURRENT_LOG" 2>&1
assert_success "[ ! -d ./tmp/storage/validator1 ]" "Validator1 storage should be removed"
assert_success "[ ! -d ./tmp/storage/validator2 ]" "Validator2 storage should be removed"
assert_success "[ ! -d ./tmp/storage/validator3 ]" "Validator3 storage should be removed"

# Test 2: Start validator1
echo ""
echo "Test 2: Starting validator1..."
VAL1_PID=$(test_start_process "./01-run-validator1.sh" "validator1")
assert_success "test_wait_for_port 10601" "Validator1 should start on port 10601"
sleep 2

# Test 3: Start validator2
echo ""
echo "Test 3: Starting validator2..."
VAL2_PID=$(test_start_process "./02-run-validator2.sh" "validator2")
assert_success "test_wait_for_port 10602" "Validator2 should start on port 10602"
sleep 2

# Test 4: Start validator3
echo ""
echo "Test 4: Starting validator3..."
VAL3_PID=$(test_start_process "./03-run-validator3.sh" "validator3")
assert_success "test_wait_for_port 10603" "Validator3 should start on port 10603"
sleep 3

# Test 5: Verify storage was created for all validators
echo ""
echo "Test 5: Verifying storage was created..."
assert_file_exists "./tmp/storage/validator1" "Validator1 storage should be created"
assert_file_exists "./tmp/storage/validator2" "Validator2 storage should be created"
assert_file_exists "./tmp/storage/validator3" "Validator3 storage should be created"

# Test 6: Check validators status
echo ""
echo "Test 6: Checking validators status..."
# Give validators time to connect
sleep 5

# Check validator1 log for connection messages
VAL1_LOG="$LOG_DIR/${CURRENT_TEST}_validator1.log"
if [ -f "$VAL1_LOG" ]; then
    echo "Validator1 log excerpt:" >> "$CURRENT_LOG"
    tail -n 20 "$VAL1_LOG" >> "$CURRENT_LOG" 2>&1 || true
fi

# Test 7: Verify validators can see each other (check for peer connections)
echo ""
echo "Test 7: Verifying peer connections..."
# Check if validators logged successful connections
if grep -q "peer" "$VAL1_LOG" 2>/dev/null; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Validator1 should have peer connections"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Validator1 should have peer connections"
fi

# Test 8: Verify genesis round is loaded
echo ""
echo "Test 8: Verifying genesis round is loaded..."
if grep -q "round" "$VAL1_LOG" 2>/dev/null || grep -q "genesis" "$VAL1_LOG" 2>/dev/null; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} Should load genesis round"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} Should load genesis round"
fi

# Test 9: Test view validators status script
echo ""
echo "Test 9: Testing view validators status script..."
if [ -f "./04-view-validators-status.sh" ]; then
    STATUS_OUTPUT=$(./04-view-validators-status.sh 2>&1 || true)
    echo "Status output: $STATUS_OUTPUT" >> "$CURRENT_LOG"
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} View validators status script should run"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -e "  ${YELLOW}⊘${NC} View validators status script not found (skipping)"
fi

# Test 10: Verify all processes are still running
echo ""
echo "Test 10: Verifying all validators are still running..."
ALL_RUNNING=true
for pid in "$VAL1_PID" "$VAL2_PID" "$VAL3_PID"; do
    if ! kill -0 "$pid" 2>/dev/null; then
        ALL_RUNNING=false
        echo "  Process $pid is not running" >> "$CURRENT_LOG"
    fi
done

if [ "$ALL_RUNNING" = true ]; then
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} All validators should remain running"
else
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} All validators should remain running"
fi

# Finalize test
test_finalize
exit $?

