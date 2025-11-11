#!/usr/bin/env bash
# Integration test for 05-mining example
# Can be run standalone or via the test runner

set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Clean up any previous test runs and stale processes
pkill -9 -f "modal node run-miner" 2>/dev/null || true
sleep 1

# Initialize test
test_init "05-mining"

# Build modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    (cd ../../../rust && cargo build --package modal)
fi

# Test 1: Clean storage
echo ""
echo "Test 1: Cleaning storage..."
./00-clean-storage.sh >> "$CURRENT_LOG" 2>&1
assert_success "[ ! -d ./tmp/storage/miner ]" "Storage should be removed"

# Test 2: Start miner
echo ""
echo "Test 2: Starting miner..."
MINER_PID=$(test_start_process "RUST_LOG=info ../../../rust/target/debug/modal node run-miner --config ./configs/miner.json" "miner")
assert_success "test_wait_for_port 10301" "Miner should start on port 10301"

# Wait for some blocks to be mined
echo "  Waiting for blocks to be mined..." >> "$CURRENT_LOG"
sleep 5

# Test 3: Wait for blocks to be mined (check log)
echo ""
echo "Test 3: Verifying blocks are being mined..."
MINER_LOG="$LOG_DIR/${CURRENT_TEST}_miner.log"
assert_success "test_wait_for_log '$MINER_LOG' 'Block .* mined' 120" "Should mine at least one block"

# Test 4: Verify storage was created
echo ""
echo "Test 4: Verifying storage was created..."
assert_file_exists "./tmp/storage/miner" "Miner storage should be created"
assert_file_exists "./tmp/storage/miner/IDENTITY" "Miner datastore should be initialized"

# Stop miner before inspecting storage (to avoid RocksDB lock)
echo "  Stopping miner to inspect storage..." >> "$CURRENT_LOG"
kill "$MINER_PID" 2>/dev/null || true
# Wait for port to be released
for i in {1..10}; do
    if ! lsof -i :10301 -sTCP:LISTEN >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
# Force kill if still running
pkill -9 -f "modal node run-miner" 2>/dev/null || true
sleep 1

# Test 5: Inspect blocks using modal node inspect (while running!)
echo ""
echo "Test 5: Inspecting running miner node with modal node inspect..."
assert_output_contains \
    "../../../rust/target/debug/modal node inspect --config ./configs/miner.json" \
    "Total Blocks" \
    "Should show block statistics from running node"

# Test 6: Verify multiple blocks were mined
echo ""
echo "Test 6: Verifying multiple blocks were mined..."
BLOCK_COUNT=$(../../../rust/target/debug/modal node inspect --config ./configs/miner.json 2>&1 | grep "Total Blocks:" | sed -E 's/.*Total Blocks: ([0-9]+).*/\1/' || echo "0")
echo "Block count: $BLOCK_COUNT" >> "$CURRENT_LOG"
assert_number "$BLOCK_COUNT" ">=" "1" "Should have mined at least 1 block"

# Test 7: Verify mining status shows in inspect output  
echo ""
echo "Test 7: Verifying mining status is reported..."
assert_output_contains \
    "../../../rust/target/debug/modal node inspect --config ./configs/miner.json --level mining" \
    "Is Mining" \
    "Should show mining status"

# Test 8: Stop miner, inspect offline, and restart (test both modes)
echo ""
echo "Test 8: Testing offline inspection after stop..."
kill "$MINER_PID" 2>/dev/null || true
sleep 2

# Now use modal node inspect in offline mode (auto-detects node is not running)
echo "Inspecting stopped node (should auto-fallback to direct datastore access)..."
assert_output_contains \
    "../../../rust/target/debug/modal node inspect --config ./configs/miner.json" \
    "Offline" \
    "Should detect node is offline"

# Test 9: Test persistence - restart and verify
echo ""
echo "Test 9: Testing persistence (restart)..."
MINER_PID=$(test_start_process "RUST_LOG=info ../../../rust/target/debug/modal node run-miner --config ./configs/miner.json" "miner-restart")
assert_success "test_wait_for_port 10301" "Miner should restart on port 10301"
sleep 3

# Verify it can mine more blocks
assert_success "test_wait_for_log '$LOG_DIR/${CURRENT_TEST}_miner-restart.log' 'Block .* mined' 120" "Should mine blocks after restart"

# Finalize test
test_finalize
exit $?

