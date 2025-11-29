#!/usr/bin/env bash
# Manual test to reproduce the infinite loop bug by crafting datastore state
# This simulates the exact scenario from testnet2

cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)

source ../test-lib.sh

set -e

echo "=== Manual Infinite Loop Bug Reproduction ==="
echo ""
echo "This test manually crafts the datastore state to force the bug:"
echo "1. Miner1 will have blocks 0-2 in datastore"
echo "2. Miner1 will try to mine block 3"
echo "3. We'll inject block 3 into the datastore (simulating gossip)"
echo "4. Miner1 should skip mining block 3, claim success, then try block 4"
echo "5. Block 4 will be rejected, miner corrects to block 3"
echo "6. INFINITE LOOP: block 3 exists → skip → increment → try 4 → reject → back to 3"
echo ""

# Clean up
echo "Step 1: Cleaning up previous runs..."
./00-clean.sh

# Create miner1
echo ""
echo "Step 2: Creating miner1..."
modal node create --dir ./tmp/miner1

# Configure miner1 (no mining delay needed, we'll control it manually)
CONFIG_FILE="./tmp/miner1/config.json"
TMP_FILE="./tmp/miner1/config.json.tmp"
PEER_ID=$(jq -r '.id' "$CONFIG_FILE")
jq '. + {
  run_miner: true,
  initial_difficulty: 1,
  listeners: ["/ip4/0.0.0.0/tcp/10401/ws"],
  miner_nominees: ["'"$PEER_ID"'"],
  bootstrappers: []
}' "$CONFIG_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$CONFIG_FILE"

echo "  Miner1 peer ID: $PEER_ID"

# Mine blocks 0, 1, 2
echo ""
echo "Step 3: Mining blocks 0, 1, 2 with miner1..."
modal node run-miner --dir ./tmp/miner1 > ./tmp/miner1-initial.log 2>&1 &
MINER1_PID=$!
echo "  Miner1 PID: $MINER1_PID"

# Wait for blocks to be mined
echo "  Waiting for genesis..."
sleep 5
while ! grep -q "Successfully mined and gossipped block 0" ./tmp/miner1-initial.log 2>/dev/null; do
    sleep 1
done
echo "  ✓ Block 0 mined"

echo "  Waiting for block 1..."
while ! grep -q "Successfully mined and gossipped block 1" ./tmp/miner1-initial.log 2>/dev/null; do
    sleep 1
done
echo "  ✓ Block 1 mined"

echo "  Waiting for block 2..."
while ! grep -q "Successfully mined and gossipped block 2" ./tmp/miner1-initial.log 2>/dev/null; do
    sleep 1
done
echo "  ✓ Block 2 mined"

# Stop miner1
echo ""
echo "Step 4: Stopping miner1..."
kill $MINER1_PID 2>/dev/null || true
sleep 2

# Check current state
echo ""
echo "Step 5: Checking miner1 chain state..."
modal node inspect --dir ./tmp/miner1

# Now manually inject block 3 into the datastore to simulate it arriving via gossip
# We'll use modal CLI or directly manipulate the datastore
echo ""
echo "Step 6: Creating miner2 to mine block 3 (simulating another miner)..."
modal node create --dir ./tmp/miner2

# Copy miner1's storage to miner2 (same chain up to block 2)
cp -r ./tmp/miner1/storage ./tmp/miner2/

# Configure miner2
CONFIG_FILE2="./tmp/miner2/config.json"
TMP_FILE2="./tmp/miner2/config.json.tmp"
PEER_ID2=$(jq -r '.id' "$CONFIG_FILE2")
jq '. + {
  run_miner: true,
  initial_difficulty: 1,
  listeners: ["/ip4/0.0.0.0/tcp/10402/ws"],
  miner_nominees: ["'"$PEER_ID2"'"],
  bootstrappers: []
}' "$CONFIG_FILE2" > "$TMP_FILE2"
mv "$TMP_FILE2" "$CONFIG_FILE2"

echo "  Miner2 peer ID: $PEER_ID2"

# Have miner2 mine block 3
echo ""
echo "Step 7: Having miner2 mine block 3..."
modal node run-miner --dir ./tmp/miner2 > ./tmp/miner2-block3.log 2>&1 &
MINER2_PID=$!

echo "  Waiting for block 3..."
while ! grep -q "Successfully mined and gossipped block 3" ./tmp/miner2-block3.log 2>/dev/null; do
    sleep 1
done
echo "  ✓ Block 3 mined by miner2"

# Stop miner2
kill $MINER2_PID 2>/dev/null || true
sleep 2

# Now copy miner2's block 3 into miner1's datastore (simulating gossip)
echo ""
echo "Step 8: Copying block 3 from miner2 to miner1 (simulating gossip)..."
# The storage should contain the blocks in RocksDB format
# We need to actually copy the database files
cp -r ./tmp/miner2/storage/* ./tmp/miner1/storage/

echo "  ✓ Block 3 now exists in miner1's datastore"

# Verify miner1 now has block 3
echo ""
echo "Step 9: Verifying miner1 has block 3..."
modal node inspect --dir ./tmp/miner1

# Now restart miner1 - it should try to mine block 4
# But we'll simulate that block 4 gets rejected by fork choice
echo ""
echo "Step 10: Restarting miner1 - should create infinite loop..."
echo ""
echo "Watch for the pattern:"
echo "  1. Mining block 4..."
echo "  2. (simulate rejection by stopping and going back)"
echo "  3. Correcting to block 3"
echo "  4. Block 3 already exists, skipping"
echo "  5. Successfully mined block 3 (FALSE!)"
echo "  6. Mining block 4... (LOOP)"
echo ""
echo "Starting miner1 (Ctrl+C to stop)..."
echo ""

# For this demonstration, we'll just show that it tries to mine block 4
# In reality, the infinite loop happens when block 4 gets rejected
modal node run-miner --dir ./tmp/miner1

# Cleanup
echo ""
echo "Cleaning up..."
./00-clean.sh

