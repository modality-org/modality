#!/usr/bin/env bash
# Static reproduction of the infinite loop bug
# 
# Strategy: Manually create a state where:
# 1. Miner has blocks 0-N in chain
# 2. Miner's current_index points to N+1 (it thinks it should mine N+1)  
# 3. Block N+1 actually exists in the datastore (received via gossip)
# 4. Miner will skip mining N+1, claim success, increment to N+2
# 5. Force rejection of N+2 by not having N+1 in the in-memory chain
# 6. This creates the infinite loop

cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)

set -e

echo "=== Static Infinite Loop Bug Reproduction ==="
echo ""

# Clean up
echo "Step 1: Cleaning up..."
rm -rf ./tmp/static-test
mkdir -p ./tmp/static-test

# Create two independent miners
echo ""
echo "Step 2: Creating two independent miners..."
modal node create --dir ./tmp/static-test/miner1
modal node create --dir ./tmp/static-test/miner2

# Configure both to mine independently (no network connection)
for miner in miner1 miner2; do
    CONFIG_FILE="./tmp/static-test/$miner/config.json"
    TMP_FILE="./tmp/static-test/$miner/config.json.tmp"
    PEER_ID=$(jq -r '.id' "$CONFIG_FILE")
    PORT=$((10401 + $(echo $miner | grep -o '[0-9]$') - 1))
    
    jq '. + {
      run_miner: true,
      initial_difficulty: 1,
      listeners: ["/ip4/0.0.0.0/tcp/'$PORT'/ws"],
      miner_nominees: ["'"$PEER_ID"'"],
      bootstrappers: []
    }' "$CONFIG_FILE" > "$TMP_FILE"
    mv "$TMP_FILE" "$CONFIG_FILE"
done

# Have miner1 mine blocks 0, 1, 2
echo ""
echo "Step 3: Miner1 mining blocks 0, 1, 2..."
timeout 60 modal node run-miner --dir ./tmp/static-test/miner1 > ./tmp/static-test/miner1.log 2>&1 &
MINER1_PID=$!

# Wait for 3 blocks
for block in 0 1 2; do
    echo "  Waiting for block $block..."
    while ! grep -q "Successfully mined and gossipped block $block" ./tmp/static-test/miner1.log 2>/dev/null; do
        sleep 1
    done
    echo "  ✓ Block $block mined"
done

kill $MINER1_PID 2>/dev/null || true
sleep 2

echo ""
echo "Step 4: Miner1 chain state:"
modal node inspect --dir ./tmp/static-test/miner1

# Have miner2 mine its OWN blocks 0, 1, 2, 3 (competing chain)
echo ""
echo "Step 5: Miner2 mining its own blocks 0, 1, 2, 3 (different chain)..."
timeout 90 modal node run-miner --dir ./tmp/static-test/miner2 > ./tmp/static-test/miner2.log 2>&1 &
MINER2_PID=$!

for block in 0 1 2 3; do
    echo "  Waiting for block $block..."
    while ! grep -q "Successfully mined and gossipped block $block" ./tmp/static-test/miner2.log 2>/dev/null; do
        sleep 1
    done
    echo "  ✓ Block $block mined"
done

kill $MINER2_PID 2>/dev/null || true
sleep 2

echo ""
echo "Step 6: Miner2 chain state:"
modal node inspect --dir ./tmp/static-test/miner2

# Now the KEY STEP: Copy miner2's block 3 into miner1's datastore
# This simulates miner1 receiving block 3 via gossip while it's trying to mine block 3
echo ""
echo "Step 7: Injecting miner2's block 3 into miner1's datastore (simulating gossip)..."
echo "  This creates the race condition state:"
echo "  - Miner1's in-memory chain: blocks 0, 1, 2 (height=2)"  
echo "  - Miner1's datastore: blocks 0, 1, 2, 3 (has block 3 from gossip)"
echo "  - Miner1 will try to mine block 3, find it exists, skip it, claim success"
echo ""

# To inject block 3, we need to copy the RocksDB data
# Copy just the new blocks from miner2 to miner1
echo "  Copying miner2's storage to miner1..."
rsync -av ./tmp/static-test/miner2/storage/ ./tmp/static-test/miner1/storage/

echo ""
echo "Step 8: Miner1 state after injection:"
modal node inspect --dir ./tmp/static-test/miner1

# Now restart miner1 - it should load block 3 from datastore
# Then try to mine block 4, but block 4 will be invalid (no parent)
# It will correct to block 3, find it exists, skip, claim success
# Then try block 4 again → INFINITE LOOP

echo ""
echo "Step 9: Restarting miner1..."
echo ""
echo "Expected behavior:"
echo "  1. Load chain from datastore (will have blocks 0, 1, 2, 3)"
echo "  2. Try to mine block 4"
echo "  3. If we simulate rejection, it corrects to block 3"
echo "  4. Block 3 exists → skip → claim success → try block 4"
echo "  5. INFINITE LOOP"
echo ""
echo "Watch the logs for 'Block 3 already exists in chain'"
echo ""
echo "Press Ctrl+C to stop when you see the loop..."
echo ""

export RUST_LOG=info
modal node run-miner --dir ./tmp/static-test/miner1

echo ""
echo "Cleaning up..."
rm -rf ./tmp/static-test

