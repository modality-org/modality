#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🧹 Cleaning up miner storage..."
echo ""

rm -rf ./tmp/storage/miner
echo "✅ Storage cleaned. Ready for a fresh mining run."
echo ""
echo "Run ./01-mine-blocks.sh to start mining again."

