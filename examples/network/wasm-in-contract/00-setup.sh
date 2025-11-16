#!/bin/bash
# Setup environment for WASM validation example

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLE_DIR="$SCRIPT_DIR"
TMP_DIR="$EXAMPLE_DIR/tmp"

echo "=== WASM Validation Example Setup ==="
echo ""

# Create tmp directory
mkdir -p "$TMP_DIR"

echo "✓ Created tmp directory: $TMP_DIR"
echo ""

# Check if modal CLI is available
if ! command -v modal &> /dev/null; then
    echo "❌ Error: modal CLI not found"
    echo "   Please install modal first"
    exit 1
fi

echo "✓ Modal CLI found: $(which modal)"
echo ""

# Check if network is running
echo "Checking if network is running..."
if pgrep -f "modal node run" > /dev/null; then
    echo "✓ Network appears to be running"
else
    echo "⚠️  Warning: No modal nodes detected"
    echo "   You may need to start a network first"
    echo "   See: examples/network/03-run-devnet3/"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "  1. Run ./01-create-contract.sh to create a contract"
echo "  2. Run ./02-upload-wasm.sh to upload WASM module"
echo "  3. Run ./03-push-contract.sh to push to network"
echo "  4. Run ./04-test-validation.sh to test"
echo ""

