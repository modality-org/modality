#!/bin/bash

# Test script to verify that two divergent chains resolve to the longest chain
# This script will:
# 1. Start node1 solo to mine some blocks (chain A)
# 2. Start node2 solo to mine some blocks (chain B)
# 3. Connect the nodes and verify that the shorter chain adopts the longer chain

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "==================================="
echo "Divergent Chain Resolution Test"
echo "==================================="
echo ""

# Clean up any existing storage
echo "1. Cleaning up existing storage..."
./00-clean-storage.sh
mkdir -p tmp/storage/node1
mkdir -p tmp/storage/node2
mkdir -p configs

# Create config for node1 (no bootstrappers - solo mining)
cat > configs/node1.json <<EOF
{
  "passfile_path": "../../../../fixtures/passfiles/node1.mod_passfile",
  "storage_path": "${SCRIPT_DIR}/tmp/storage/node1",
  "listeners": ["/ip4/0.0.0.0/tcp/4041/ws"],
  "bootstrappers": [],
  "run_miner": true,
  "miner_nominees": ["node1"]
}
EOF

# Create config for node2 (no bootstrappers - solo mining)
cat > configs/node2.json <<EOF
{
  "passfile_path": "../../../../fixtures/passfiles/node2.mod_passfile",
  "storage_path": "${SCRIPT_DIR}/tmp/storage/node2",
  "listeners": ["/ip4/0.0.0.0/tcp/4042/ws"],
  "bootstrappers": [],
  "run_miner": true,
  "miner_nominees": ["node2"]
}
EOF

echo "✓ Config files created"
echo ""

# Start node1 in solo mode
echo "2. Starting node1 in solo mode to mine 10 blocks..."
RUST_LOG=info ../../../build/bin/modality node run-miner \
  --config configs/node1.json &
NODE1_PID=$!
echo "   Node1 PID: $NODE1_PID"

# Wait for node1 to mine some blocks
echo "   Waiting 15 seconds for node1 to mine blocks..."
sleep 15

# Stop node1
echo "   Stopping node1..."
kill $NODE1_PID 2>/dev/null || true
wait $NODE1_PID 2>/dev/null || true
sleep 2

# Check node1's block count
echo ""
echo "   Checking node1's blockchain:"
NODE1_BLOCKS=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node1.json" --detailed 2>/dev/null | grep "^  Block #" | wc -l | tr -d ' ')
echo "   Node1 has mined: $NODE1_BLOCKS blocks"

echo ""
echo "3. Starting node2 in solo mode to mine 20 blocks..."
RUST_LOG=info ../../../build/bin/modality node run-miner \
  --config configs/node2.json &
NODE2_PID=$!
echo "   Node2 PID: $NODE2_PID"

# Wait for node2 to mine more blocks than node1
echo "   Waiting 25 seconds for node2 to mine more blocks..."
sleep 25

# Stop node2
echo "   Stopping node2..."
kill $NODE2_PID 2>/dev/null || true
wait $NODE2_PID 2>/dev/null || true
sleep 2

# Check node2's block count
echo ""
echo "   Checking node2's blockchain:"
NODE2_BLOCKS=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node2.json" --detailed 2>/dev/null | grep "^  Block #" | wc -l | tr -d ' ')
echo "   Node2 has mined: $NODE2_BLOCKS blocks"

echo ""
echo "4. Chain divergence created:"
echo "   - Node1: $NODE1_BLOCKS blocks (shorter chain)"
echo "   - Node2: $NODE2_BLOCKS blocks (longer chain)"
echo ""

if [ "$NODE2_BLOCKS" -le "$NODE1_BLOCKS" ]; then
    echo "❌ SETUP FAILED: Node2 should have more blocks than node1"
    echo "   Node1: $NODE1_BLOCKS, Node2: $NODE2_BLOCKS"
    exit 1
fi

# Now restart both nodes WITH each other as bootstrappers so they connect
echo "5. Restarting both nodes connected to each other..."
echo ""

# Update configs to include bootstrappers with full peer IDs
cat > configs/node1-connected.json <<EOF
{
  "passfile_path": "../../../../fixtures/passfiles/node1.mod_passfile",
  "storage_path": "${SCRIPT_DIR}/tmp/storage/node1",
  "listeners": ["/ip4/0.0.0.0/tcp/4041/ws"],
  "bootstrappers": ["/ip4/127.0.0.1/tcp/4042/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB"],
  "run_miner": true,
  "miner_nominees": ["node1"]
}
EOF

cat > configs/node2-connected.json <<EOF
{
  "passfile_path": "../../../../fixtures/passfiles/node2.mod_passfile",
  "storage_path": "${SCRIPT_DIR}/tmp/storage/node2",
  "listeners": ["/ip4/0.0.0.0/tcp/4042/ws"],
  "bootstrappers": ["/ip4/127.0.0.1/tcp/4041/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"],
  "run_miner": true,
  "miner_nominees": ["node2"]
}
EOF

# Start both nodes connected
echo "   Starting node2 (longer chain)..."
RUST_LOG=info ../../../build/bin/modality node run-miner \
  --config configs/node2-connected.json > tmp/node2.log 2>&1 &
NODE2_PID=$!
echo "   Node2 PID: $NODE2_PID"

sleep 2

echo "   Starting node1 (shorter chain)..."
RUST_LOG=info ../../../build/bin/modality node run-miner \
  --config configs/node1-connected.json > tmp/node1.log 2>&1 &
NODE1_PID=$!
echo "   Node1 PID: $NODE1_PID"

echo ""
echo "6. Waiting 30 seconds for chains to sync and resolve..."
sleep 30

# Stop both nodes
echo ""
echo "   Stopping both nodes..."
kill $NODE1_PID 2>/dev/null || true
kill $NODE2_PID 2>/dev/null || true
wait $NODE1_PID 2>/dev/null || true
wait $NODE2_PID 2>/dev/null || true
sleep 2

# Check the final state of both nodes
echo ""
echo "7. Checking final blockchain state:"
echo ""

NODE1_FINAL=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node1-connected.json" --detailed 2>/dev/null | grep "^  Block #" | wc -l | tr -d ' ')
NODE2_FINAL=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node2-connected.json" --detailed 2>/dev/null | grep "^  Block #" | wc -l | tr -d ' ')

echo "   Node1 final: $NODE1_FINAL blocks"
echo "   Node2 final: $NODE2_FINAL blocks"

# Get the last few block hashes from each node to verify they're on the same chain
echo ""
echo "   Checking if chains have converged (comparing recent block hashes)..."
NODE1_HASH=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node1-connected.json" --detailed --limit 100 2>/dev/null | \
  grep "^  Block #" | tail -3 | head -1 | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2 || echo "none")
NODE2_HASH=$(RUST_LOG=error ../../../build/bin/modality net storage \
  --config "${SCRIPT_DIR}/configs/node2-connected.json" --detailed --limit 100 2>/dev/null | \
  grep "^  Block #" | tail -3 | head -1 | grep -o "hash: [a-f0-9]*" | cut -d' ' -f2 || echo "none")

echo "   Node1 recent block hash: ${NODE1_HASH:0:16}..."
echo "   Node2 recent block hash: ${NODE2_HASH:0:16}..."

echo ""
echo "==================================="
echo "Test Results"
echo "==================================="
echo ""
echo "Initial state:"
echo "  - Node1: $NODE1_BLOCKS blocks (shorter chain)"
echo "  - Node2: $NODE2_BLOCKS blocks (longer chain)"
echo ""
echo "Final state:"
echo "  - Node1: $NODE1_FINAL blocks"
echo "  - Node2: $NODE2_FINAL blocks"
echo ""

# Check if node1 adopted node2's longer chain
if [ "$NODE1_FINAL" -ge "$NODE2_BLOCKS" ] && [ "$NODE1_HASH" == "$NODE2_HASH" ]; then
    echo "✅ SUCCESS: Node1 adopted the longer chain!"
    echo "   Both nodes now have the same chain with matching block hashes."
    echo ""
    echo "Logs are available at:"
    echo "  - tmp/node1.log"
    echo "  - tmp/node2.log"
    exit 0
elif [ "$NODE1_FINAL" -ge "$NODE2_BLOCKS" ]; then
    echo "⚠️  PARTIAL SUCCESS: Node1 has enough blocks but hashes don't match"
    echo "   This could indicate the chains haven't fully synchronized yet."
    echo ""
    echo "Check the logs for more details:"
    echo "  - tmp/node1.log"
    echo "  - tmp/node2.log"
    exit 1
else
    echo "❌ FAILURE: Node1 did not adopt the longer chain"
    echo "   Node1 should have at least $NODE2_BLOCKS blocks but has $NODE1_FINAL"
    echo ""
    echo "Check the logs for sync/reorg errors:"
    echo "  - tmp/node1.log"
    echo "  - tmp/node2.log"
    echo ""
    echo "Common issues:"
    echo "  - Chain sync not triggered"
    echo "  - Chain reorg logic not working"
    echo "  - Gossip not propagating blocks"
    exit 1
fi

