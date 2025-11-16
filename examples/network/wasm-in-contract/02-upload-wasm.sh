#!/bin/bash
# Upload a WASM validation module to the contract

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP_DIR="$SCRIPT_DIR/tmp"
CONTRACT_DIR="$TMP_DIR/wasm-contract"

echo "=== Uploading WASM Module ==="
echo ""

# Check if contract exists
if [ ! -f "$TMP_DIR/contract_id.txt" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first"
    exit 1
fi

CONTRACT_ID=$(cat "$TMP_DIR/contract_id.txt")
echo "Contract ID: $CONTRACT_ID"
echo ""

# Create a minimal valid WASM module for testing
# This is the smallest valid WASM module (just magic number and version)
WASM_FILE="$TMP_DIR/minimal.wasm"
echo "Creating minimal WASM module..."

# Write WASM magic number and version
printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > "$WASM_FILE"

echo "✓ Created minimal WASM module: $WASM_FILE"
echo "  Size: $(wc -c < "$WASM_FILE") bytes"
echo ""

# Upload WASM module with default gas limit
echo "Uploading WASM module via modal contract wasm-upload..."
modal contract wasm-upload \
    --dir "$CONTRACT_DIR" \
    --wasm-file "$WASM_FILE" \
    --module-name "validator" \
    --output json > "$TMP_DIR/wasm-upload-output.json"

cat "$TMP_DIR/wasm-upload-output.json"
echo ""

COMMIT_ID=$(cat "$TMP_DIR/wasm-upload-output.json" | grep -o '"commit_id":"[^"]*"' | cut -d'"' -f4)

echo "✓ WASM module uploaded"
echo "  Commit ID: $COMMIT_ID"
echo "  Module: validator"
echo "  Path: /validator.wasm"
echo ""

echo "=== WASM Upload Complete ==="
echo ""
echo "The WASM module has been added to the contract as a POST action."
echo "Next: Run ./03-push-contract.sh to push to network"
echo ""

