#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🧹 Cleaning up miner node directory..."
echo ""

rm -rf ./tmp/miner
echo "✅ Miner node directory cleaned. Ready for a fresh mining run."
echo ""
echo "Run ./01-mine-blocks.sh to create and start mining again."


