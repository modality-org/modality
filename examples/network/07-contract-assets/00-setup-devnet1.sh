#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0: Setup devnet1"
echo "================================================"
echo ""

# Clean up any previous state
echo "Cleaning up previous state..."
rm -rf data/
rm -rf tmp/
mkdir -p data/alice
mkdir -p data/bob
mkdir -p tmp/node1
mkdir -p tmp/test-logs

echo "✅ Setup complete!"
echo ""
echo "Directories created:"
echo "  - data/alice (Alice's contract)"
echo "  - data/bob (Bob's contract)"
echo "  - tmp/node1 (devnet1 validator)"
echo ""

