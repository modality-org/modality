#!/usr/bin/env bash
cd $(dirname -- "$0")

# Create 3 epochs of test miner blocks in node1's datastore
# This uses the Rust example from modal-datastore

echo "Setting up node1 with 3 epochs of miner blocks (120 blocks total)..."
echo ""

# Determine storage path
STORAGE_PATH="./tmp/storage/node1"
mkdir -p "$STORAGE_PATH"

# Get absolute path
ABS_STORAGE_PATH=$(cd "$STORAGE_PATH" && pwd)

# Navigate to rust directory and run the create_test_blocks example
cd ../../../rust

echo "Running create_test_blocks example..."
cargo run --package modal-datastore --example create_test_blocks --quiet -- "$ABS_STORAGE_PATH" 120

cd - > /dev/null

echo ""
echo "✅ Setup complete! Node1 is ready with 3 epochs of miner blocks."
echo "   Run ./01-run-node1.sh to start the node."

