#!/usr/bin/env bash
# Integration test for 05-mining example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Clean up any previous test runs and stale processes
modal local killall-nodes --dir . --force 2>/dev/null || true
sleep 1

# Initialize test
test_init "05-mining"

# Build modal CLI if needed
if ! command -v modal &> /dev/null; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
    export PATH="../../../rust/target/debug:$PATH"
fi

# Test 1: Clean node directory
echo ""
echo "Test 1: Cleaning node directory..."
./00-clean-storage.sh >> "$CURRENT_LOG" 2>&1
assert_success "[ ! -d ./tmp/miner ]" "Node directory should be removed"

# Test 2: Create miner node
echo ""
echo "Test 2: Creating miner node..."
modal node create \
    --dir ./tmp/miner \
    --network devnet1 >> "$CURRENT_LOG" 2>&1

# Configure node for mining
CONFIG_FILE="./tmp/miner/config.json"
TMP_FILE="./tmp/miner/config.json.tmp"
if command -v jq &> /dev/null; then
    jq '. + {run_miner: true, status_port: 8080, initial_difficulty: 1, listeners: ["/ip4/0.0.0.0/tcp/10301/ws"]}' "$CONFIG_FILE" > "$TMP_FILE"
    mv "$TMP_FILE" "$CONFIG_FILE"
fi

assert_file_exists "./tmp/miner/config.json" "Node config should be created"
assert_file_exists "./tmp/miner/node.passfile" "Node passfile should be created"

# Test 3: Start miner
echo ""
echo "Test 3: Starting miner..."
MINER_PID=$(test_start_process "RUST_LOG=info modal node run-miner --dir ./tmp/miner" "miner")
assert_success "test_wait_for_port 10301" "Miner should start on port 10301"

# Wait for some blocks to be mined
echo "  Waiting for blocks to be mined..." >> "$CURRENT_LOG"
sleep 5

# Test 4: Wait for blocks to be mined (check log)
echo ""
echo "Test 4: Verifying blocks are being mined..."
MINER_LOG="$LOG_DIR/${CURRENT_TEST}_miner.log"
assert_success "test_wait_for_log '$MINER_LOG' 'Block .* mined' 120" "Should mine at least one block"

# Test 5: Verify storage was created
echo ""
echo "Test 5: Verifying storage was created..."
assert_file_exists "./tmp/miner/storage" "Miner storage should be created"
assert_file_exists "./tmp/miner/storage/IDENTITY" "Miner datastore should be initialized"

# Test 6: Inspect blocks using modal node inspect (while running!)
echo ""
echo "Test 6: Inspecting running miner node with modal node inspect..."
assert_output_contains \
    "modal node inspect --dir ./tmp/miner" \
    "Total Blocks" \
    "Should show block statistics from running node"

# Test 7: Verify multiple blocks were mined
echo ""
echo "Test 7: Verifying multiple blocks were mined..."
BLOCK_COUNT=$(modal node inspect --dir ./tmp/miner 2>&1 | grep "Total Blocks:" | sed -E 's/.*Total Blocks: ([0-9]+).*/\1/' || echo "0")
echo "Block count: $BLOCK_COUNT" >> "$CURRENT_LOG"
assert_number "$BLOCK_COUNT" ">=" "1" "Should have mined at least 1 block"

# Test 8: Verify mining status shows in inspect output  
echo ""
echo "Test 8: Verifying mining status is reported..."
assert_output_contains \
    "modal node inspect --dir ./tmp/miner mining" \
    "Is Mining" \
    "Should show mining status"

# Test 9: Stop miner, inspect offline, and restart (test both modes)
echo ""
echo "Test 9: Testing offline inspection after stop..."
kill "$MINER_PID" 2>/dev/null || true
sleep 2

# Now use modal node inspect in offline mode (auto-detects node is not running)
echo "Inspecting stopped node (should auto-fallback to direct datastore access)..."
assert_output_contains \
    "modal node inspect --dir ./tmp/miner" \
    "Offline" \
    "Should detect node is offline"

# Test 10: Testing persistence (restart)...
echo ""
echo "Test 10: Testing persistence (restart)..."
MINER_PID=$(test_start_process "RUST_LOG=info modal node run-miner --dir ./tmp/miner" "miner-restart")
assert_success "test_wait_for_port 10301" "Miner should restart on port 10301"
sleep 3

# Verify it can mine more blocks
assert_success "test_wait_for_log '$LOG_DIR/${CURRENT_TEST}_miner-restart.log' 'Block .* mined' 120" "Should mine blocks after restart"

# Finalize test
test_finalize
exit $?
