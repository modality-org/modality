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

# Build the modality CLI if needed
if [ ! -f "../../../rust/target/debug/modality" ]; then
    echo "Building modality CLI..."
    cd ../../../rust
    cargo build --package modality
    cd - > /dev/null
fi

../../../rust/target/debug/modality net storage --config ./configs/miner.json --detailed

