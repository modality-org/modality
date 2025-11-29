#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 0: Setup"
echo "================================================"
echo ""

# Clean up any previous state
echo "Cleaning up previous state..."
rm -rf tmp/alice tmp/bob tmp/send-commit-id.txt
mkdir -p tmp/alice
mkdir -p tmp/bob

echo "✅ Setup complete!"
echo ""
echo "Directories created:"
echo "  - tmp/alice (Alice's contract)"
echo "  - tmp/bob (Bob's contract)"
echo ""

