#!/bin/bash
# Create a new contract for WASM validation example

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP_DIR="$SCRIPT_DIR/tmp"
CONTRACT_DIR="$TMP_DIR/wasm-contract"

echo "=== Creating Contract ==="
echo ""

# Create contract
echo "Creating new contract at: $CONTRACT_DIR"
modal contract create --dir "$CONTRACT_DIR" --output json > "$TMP_DIR/create-output.json"

CONTRACT_ID=$(cat "$TMP_DIR/create-output.json" | grep -o '"contract_id":"[^"]*"' | cut -d'"' -f4)

echo "✓ Contract created"
echo "  Contract ID: $CONTRACT_ID"
echo "  Directory: $CONTRACT_DIR"
echo ""

# Save contract ID for later scripts
echo "$CONTRACT_ID" > "$TMP_DIR/contract_id.txt"

echo "=== Contract Created ==="
echo ""
echo "Next: Run ./02-upload-wasm.sh to upload WASM module"
echo ""

