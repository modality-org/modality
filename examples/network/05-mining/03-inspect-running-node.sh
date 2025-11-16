#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🔍 Inspecting Running Miner Node"
echo "================================="
echo ""
echo "This demonstrates querying a RUNNING node's state without stopping it."
echo ""

if [ ! -d "./tmp/miner" ]; then
    echo "❌ No miner node found. Run 01-mine-blocks.sh first."
    exit 1
fi

# Check if miner is running
if ! lsof -i :10301 > /dev/null 2>&1; then
    echo "❌ Miner not running on port 10301. Run 01-mine-blocks.sh first."
    exit 1
fi

echo "✅ Miner is running"
echo ""

# Build the modal CLI if needed
if ! command -v modal &> /dev/null; then
    echo "Building modal CLI..."
    cd ../../../rust
    cargo build --package modal
    cd - > /dev/null
    export PATH="$(cd ../../../rust/target/debug && pwd):$PATH"
fi

echo "📊 Basic Inspection (default level)"
echo "------------------------------------"
modal node inspect --dir ./tmp/miner
echo ""

echo "📊 Block Information"
echo "------------------------------------"
modal node inspect --dir ./tmp/miner blocks
echo ""

echo "⛏️  Mining Information"
echo "------------------------------------"
modal node inspect --dir ./tmp/miner mining
echo ""

echo "💡 TIP: The node keeps running while being inspected!"
echo "💡 Compare this to 'modal net storage' which requires direct datastore access (offline query)"
echo ""


