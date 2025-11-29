#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0: Setup devnet1"
echo "================================================"
echo ""

# Clean up any previous state
echo "Cleaning up previous state..."
rm -rf tmp/
mkdir -p tmp/alice
mkdir -p tmp/bob
mkdir -p tmp/node1
mkdir -p tmp/test-logs

echo "✅ Setup complete!"
echo ""
echo "Directories created:"
echo "  - tmp/alice (Alice's contract)"
echo "  - tmp/bob (Bob's contract)"
echo "  - tmp/node1 (devnet1 validator)"
echo ""

