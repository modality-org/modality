#!/usr/bin/env bash
# Integration test for 10-hybrid-devnet1 example
# Tests single miner/validator hybrid consensus

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "10-hybrid-devnet1"

echo ""
echo "============================================"
echo "Testing Hybrid Devnet1 - Single Miner/Validator"
echo "============================================"

# Test 1: Start hybrid node
echo ""
echo "Test 1: Starting hybrid miner/validator node..."
NODE1_PID=$(test_start_process "cd $(pwd) && ./01-run-hybrid-node.sh" "node1")
assert_success "test_wait_for_port 10111" "Node should start on port 10111"
sleep 3

# Test 2: Verify node is mining
echo ""
echo "Test 2: Verifying node is mining blocks..."
echo "Waiting for first block to be mined (may take 30-60 seconds)..."
sleep 60 # Wait for at least one block to be mined
NODE1_LOG="$LOG_DIR/${CURRENT_TEST}_node1.log"
if [ -f "$NODE1_LOG" ]; then
    # Check for mining activity
    if grep -q "Mined block" "$NODE1_LOG"; then
        echo "✅ PASS: Node is mining blocks"
    else
        echo "❌ FAIL: No mining activity detected after 60s"
        echo "Last 20 lines of log:"
        tail -20 "$NODE1_LOG"
        test_fail "Node should be mining blocks"
    fi
fi

# Test 3: Wait for epoch 2 and verify validator activation
echo ""
echo "Test 3: Waiting for epoch 2 and validator activation..."
echo "Mining 80+ blocks to reach epoch 2... (this may take 15-20 minutes with difficulty=1)"

# Wait up to 25 minutes for epoch 2
TIMEOUT=1500
ELAPSED=0
EPOCH2_REACHED=false

while [ $ELAPSED -lt $TIMEOUT ]; do
    sleep 10
    ELAPSED=$((ELAPSED + 10))
    
    if [ -f "$NODE1_LOG" ]; then
        # Check if we've reached epoch 2
        if grep -q "EPOCH 2 STARTED" "$NODE1_LOG"; then
            echo "✅ Epoch 2 reached after ${ELAPSED}s"
            EPOCH2_REACHED=true
            break
        fi
        
        # Show progress
        LATEST_BLOCK=$(grep "Mined block" "$NODE1_LOG" | tail -1 | grep -oE "block [0-9]+" | grep -oE "[0-9]+")
        if [ -n "$LATEST_BLOCK" ]; then
            echo "Progress: Block $LATEST_BLOCK/80+ (${ELAPSED}s elapsed)"
        fi
    fi
done

if [ "$EPOCH2_REACHED" = false ]; then
    echo "❌ FAIL: Did not reach epoch 2 within ${TIMEOUT}s"
    test_fail "Should reach epoch 2"
fi

# Give validator a moment to start after epoch transition
sleep 5

# Test 4: Verify epoch transition was broadcast
echo ""
echo "Test 4: Checking epoch transition broadcast..."
if grep -q "Broadcasted epoch 2 transition" "$NODE1_LOG"; then
    echo "✅ PASS: Epoch transition was broadcast"
else
    echo "❌ FAIL: Epoch transition not broadcast"
    test_fail "Epoch transition should be broadcast"
fi

# Test 5: Verify validator detected the transition
echo ""
echo "Test 5: Checking validator detected epoch transition..."
if grep -q "Epoch transition detected" "$NODE1_LOG"; then
    echo "✅ PASS: Validator detected epoch transition"
else
    echo "❌ FAIL: Validator did not detect transition"
    test_fail "Validator should detect epoch transition"
fi

# Test 6: Verify validator set was generated
echo ""
echo "Test 6: Checking validator set generation..."
if grep -q "Validator set for epoch 2" "$NODE1_LOG"; then
    echo "✅ PASS: Validator set generated from epoch 0 nominations"
else
    echo "❌ FAIL: Validator set not generated"
    test_fail "Validator set should be generated"
fi

# Test 7: Verify node is in validator set and started consensus
echo ""
echo "Test 7: Checking if node started Shoal consensus..."
if grep -q "This node IS a validator for epoch 2" "$NODE1_LOG"; then
    echo "✅ PASS: Node recognized as validator for epoch 2"
else
    echo "❌ FAIL: Node not recognized as validator"
    test_fail "Node should be validator for epoch 2"
fi

if grep -q "Starting Shoal consensus" "$NODE1_LOG"; then
    echo "✅ PASS: Shoal consensus started"
else
    echo "❌ FAIL: Shoal consensus not started"
    test_fail "Shoal consensus should start"
fi

# Test 8: Verify node continues mining after becoming validator
echo ""
echo "Test 8: Verifying continued mining after validator activation..."
sleep 10
MINING_AFTER=$(grep -c "Mined block" "$NODE1_LOG" || true)
if [ "$MINING_AFTER" -gt 80 ]; then
    echo "✅ PASS: Node continues mining as validator (${MINING_AFTER} blocks total)"
else
    echo "❌ FAIL: Mining stopped after validator activation"
    test_fail "Node should continue mining"
fi

echo ""
echo "============================================"
echo "All tests passed! ✅"
echo "============================================"
echo ""
echo "Hybrid consensus working:"
echo "  ✓ Node mines blocks in epochs 0-1"
echo "  ✓ Validator set generated from epoch 0 at epoch 2"
echo "  ✓ Node becomes validator at epoch 2"
echo "  ✓ Mining and validation run concurrently"

# Test summary
test_summary

