#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "📊 Inspecting Mined Blocks"
echo "=========================="
echo ""

if [ ! -d "./tmp/storage/miner" ]; then
    echo "❌ No miner storage found. Run 01-mine-blocks.sh first."
    exit 1
fi

# Build the modal CLI if needed
if [ ! -f "../../../rust/target/debug/modal" ]; then
    echo "Building modal CLI..."
    cd ../../../rust
    cargo build --package modal
    cd - > /dev/null
fi

../../../rust/target/debug/modal net storage --config ./configs/miner.json --detailed

