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

# Build modal CLI if needed
command -v modal &> /dev/null || rebuild

modal net storage --dir ./tmp/miner --detailed


