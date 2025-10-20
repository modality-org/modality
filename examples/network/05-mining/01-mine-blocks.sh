#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🔨 Starting miner node..."
echo ""
echo "This will mine blocks continuously and demonstrate:"
echo "- Mining with proper difficulty calculation"
echo "- Difficulty adjustment after each epoch (40 blocks)"
echo "- Persistent blockchain state"
echo ""
echo "Storage: $(pwd)/tmp/storage/miner"
echo ""
echo "Press Ctrl+C to stop mining"
echo ""

# Build the modality CLI if needed
if [ ! -f "../../../rust/target/debug/modality" ]; then
    echo "Building modality CLI..."
    cd ../../../rust
    cargo build --package modality
    cd - > /dev/null
fi

# Clean up old storage if requested
if [ "$1" == "--clean" ]; then
    echo "Cleaning up old storage..."
    rm -rf ./tmp/storage/miner
    echo ""
fi

# Run the miner
export RUST_LOG=info,modality_network_node=info
../../../rust/target/debug/modality node run-miner --config ./configs/miner.json

