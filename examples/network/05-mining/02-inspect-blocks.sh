#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "📊 Inspecting Mined Blocks"
echo "=========================="
echo ""

if [ ! -d "./tmp/miner" ]; then
    echo "❌ No miner node found. Run 01-mine-blocks.sh first."
    exit 1
fi

# Build the modal CLI if needed
if ! command -v modal &> /dev/null; then
    echo "Building modal CLI..."
    cd ../../../rust
    cargo build --package modal
    cd - > /dev/null
    export PATH="$(cd ../../../rust/target/debug && pwd):$PATH"
fi

modal net storage --dir ./tmp/miner --detailed


