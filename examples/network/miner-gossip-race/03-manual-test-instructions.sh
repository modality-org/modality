#!/usr/bin/env bash
# Quick manual test - view logs from both miners side by side

cd $(dirname -- "$0")

echo "This script helps you observe the race condition in real-time"
echo ""
echo "Instructions:"
echo "1. Open two terminal windows"
echo "2. In terminal 1, run: ./01-run-miner1.sh"
echo "3. In terminal 2, run: ./02-run-miner2.sh"
echo "4. Watch for these patterns in miner2's output:"
echo "   - '⚠️  Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)'"
echo "   - '⛏️  Correcting mining index from X to Y after error'"
echo "   - 'Block X already exists in chain, skipping mining'"
echo ""
echo "To stop: Press Ctrl+C in each terminal"
echo ""
echo "To clean up: ./00-clean.sh"

