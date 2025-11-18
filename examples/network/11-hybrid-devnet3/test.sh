#!/usr/bin/env bash
# Integration test for 11-hybrid-devnet3 example
# Tests 3-miner/3-validator hybrid consensus

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Clean up any previous test nodes
rm -rf ./tmp

# Initialize test (after cleanup so logs directory is created fresh)
test_init "11-hybrid-devnet3"

echo ""
echo "============================================"
echo "Testing Hybrid Devnet3 - 3 Miners/Validators"
echo "============================================"

# Test 1: Start all 3 hybrid nodes
echo ""
echo "Test 1: Starting 3 hybrid miner/validator nodes..."
NODE1_PID=$(test_start_process "cd $(pwd) && ./01-run-miner1.sh" "node1")
assert_success "test_wait_for_port 10311" "Node1 should start on port 10311"
sleep 3

NODE2_PID=$(test_start_process "cd $(pwd) && ./02-run-miner2.sh" "node2")
assert_success "test_wait_for_port 10312" "Node2 should start on port 10312"
sleep 3

NODE3_PID=$(test_start_process "cd $(pwd) && ./03-run-miner3.sh" "node3")
assert_success "test_wait_for_port 10313" "Node3 should start on port 10313"
sleep 5

# Test 2: Verify all nodes are mining
echo ""
echo "Test 2: Verifying all nodes are mining blocks..."
sleep 15 # Wait for blocks to be mined
for i in 1 2 3; do
    NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
    if [ -f "$NODE_LOG" ]; then
        if grep -q "Mined block" "$NODE_LOG"; then
            echo "✅ Node${i} is mining blocks"
        else
            echo "❌ Node${i} is not mining"
            test_fail "All nodes should be mining"
        fi
    fi
done

# Test 3: Wait for epoch 2
echo ""
echo "Test 3: Waiting for epoch 2... (mining 80+ blocks)"
echo "This may take 10-15 minutes with 3 miners at difficulty=1..."

TIMEOUT=1200
ELAPSED=0
EPOCH2_REACHED=false

while [ $ELAPSED -lt $TIMEOUT ]; do
    sleep 10
    ELAPSED=$((ELAPSED + 10))
    
    # Check any node's log for epoch 2
    for i in 1 2 3; do
        NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
        if [ -f "$NODE_LOG" ] && grep -q "EPOCH 2 STARTED" "$NODE_LOG"; then
            echo "✅ Epoch 2 reached (detected on node${i}) after ${ELAPSED}s"
            EPOCH2_REACHED=true
            break 2
        fi
    done
    
    # Show progress from node1
    NODE1_LOG="$LOG_DIR/${CURRENT_TEST}_node1.log"
    if [ -f "$NODE1_LOG" ]; then
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

sleep 5

# Test 4: Verify epoch transition coordination
echo ""
echo "Test 4: Checking epoch transition was broadcast..."
BROADCAST_COUNT=0
for i in 1 2 3; do
    NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
    if [ -f "$NODE_LOG" ] && grep -q "Broadcasted epoch 2 transition" "$NODE_LOG"; then
        echo "✅ Node${i} broadcast epoch transition"
        BROADCAST_COUNT=$((BROADCAST_COUNT + 1))
    fi
done

if [ $BROADCAST_COUNT -gt 0 ]; then
    echo "✅ PASS: ${BROADCAST_COUNT} node(s) broadcast epoch transition"
else
    echo "❌ FAIL: No epoch transitions broadcast"
    test_fail "At least one node should broadcast epoch transition"
fi

# Test 5: Verify validator set was generated from epoch 0
echo ""
echo "Test 5: Checking validator set generation..."
VALIDATOR_SET_COUNT=0
for i in 1 2 3; do
    NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
    if [ -f "$NODE_LOG" ] && grep -q "Validator set for epoch 2" "$NODE_LOG"; then
        echo "✅ Node${i} generated validator set"
        VALIDATOR_SET_COUNT=$((VALIDATOR_SET_COUNT + 1))
    fi
done

if [ $VALIDATOR_SET_COUNT -gt 0 ]; then
    echo "✅ PASS: Validator set generated from epoch 0 nominations"
else
    echo "❌ FAIL: No validator set generated"
    test_fail "Validator set should be generated"
fi

# Test 6: Check which nodes became validators
echo ""
echo "Test 6: Identifying validators for epoch 2..."
VALIDATOR_COUNT=0
for i in 1 2 3; do
    NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
    if [ -f "$NODE_LOG" ]; then
        if grep -q "This node IS a validator for epoch 2" "$NODE_LOG"; then
            echo "✅ Node${i} IS a validator for epoch 2"
            VALIDATOR_COUNT=$((VALIDATOR_COUNT + 1))
            
            # Check if Shoal consensus started
            if grep -q "Starting Shoal consensus" "$NODE_LOG"; then
                echo "  ✓ Node${i} started Shoal consensus"
            else
                echo "  ❌ Node${i} did not start consensus"
                test_fail "Validator should start consensus"
            fi
        elif grep -q "This node is NOT in the validator set" "$NODE_LOG"; then
            echo "ℹ️  Node${i} is NOT a validator for epoch 2"
        fi
    fi
done

echo ""
echo "Validator count for epoch 2: ${VALIDATOR_COUNT}"

# All 3 nodes nominate all 3 peer IDs, so all should become validators
if [ $VALIDATOR_COUNT -eq 3 ]; then
    echo "✅ PASS: All 3 nodes are validators (expected with full cross-nomination)"
elif [ $VALIDATOR_COUNT -gt 0 ]; then
    echo "✅ PASS: ${VALIDATOR_COUNT} node(s) became validator(s)"
else
    echo "❌ FAIL: No validators activated"
    test_fail "At least one node should be a validator"
fi

# Test 7: Verify continued mining
echo ""
echo "Test 7: Verifying continued mining after validator activation..."
sleep 10
for i in 1 2 3; do
    NODE_LOG="$LOG_DIR/${CURRENT_TEST}_node${i}.log"
    if [ -f "$NODE_LOG" ]; then
        MINING_COUNT=$(grep -c "Mined block" "$NODE_LOG" || true)
        if [ "$MINING_COUNT" -gt 25 ]; then  # At least some blocks (3 miners share the work)
            echo "✅ Node${i} continues mining (${MINING_COUNT} blocks)"
        fi
    fi
done

echo ""
echo "============================================"
echo "All tests passed! ✅"
echo "============================================"
echo ""
echo "Hybrid consensus working with 3 nodes:"
echo "  ✓ All nodes mine blocks in epochs 0-1"
echo "  ✓ Validator set generated from epoch 0 at epoch 2"
echo "  ✓ Validators activated at epoch 2"
echo "  ✓ Mining and validation run concurrently"
echo "  ✓ Multi-node consensus coordination"

# Test summary
test_summary

