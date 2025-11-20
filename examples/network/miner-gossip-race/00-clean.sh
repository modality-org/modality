#!/usr/bin/env bash
cd $(dirname -- "$0")

echo "Cleaning up miner-gossip-race test..."

# Kill any running nodes in this directory tree
modal local killall-nodes --dir . --force 2>/dev/null || true

# Wait for processes to be killed
sleep 1

# Remove tmp directory
rm -rf ./tmp

echo "✓ Cleanup complete"

