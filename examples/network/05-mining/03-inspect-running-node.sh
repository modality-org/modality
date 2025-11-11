#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🔍 Inspecting Running Miner Node"
echo "================================="
echo ""
echo "This demonstrates querying a RUNNING node's state without stopping it."
echo ""

# Check if miner is running
if ! lsof -i :10301 > /dev/null 2>&1; then
    echo "❌ Miner not running on port 10301. Run 01-mine-blocks.sh first."
    exit 1
fi

echo "✅ Miner is running"
echo ""

# Build the modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    cd ../../../rust
    cargo build --package modal
    cd - > /dev/null
fi

echo "📊 Basic Inspection (default level)"
echo "------------------------------------"
../../../rust/target/debug/modal node inspect --config ./configs/miner.json
echo ""

echo "📊 Detailed Datastore Inspection"
echo "------------------------------------"
../../../rust/target/debug/modal node inspect --config ./configs/miner.json --level datastore
echo ""

echo "⛏️  Mining Information"
echo "------------------------------------"
../../../rust/target/debug/modal node inspect --config ./configs/miner.json --level mining
echo ""

echo "💡 TIP: The node keeps running while being inspected!"
echo "💡 Compare this to 'modal net storage' which requires direct datastore access (offline query)"
echo ""

