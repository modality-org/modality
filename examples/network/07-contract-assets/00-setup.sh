#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0: Setup"
echo "================================================"
echo ""

# Clean up any previous state
echo "Cleaning up previous state..."
rm -rf data/
mkdir -p data/alice
mkdir -p data/bob

echo "✅ Setup complete!"
echo ""
echo "Directories created:"
echo "  - data/alice (Alice's contract)"
echo "  - data/bob (Bob's contract)"
echo ""

