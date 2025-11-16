#!/bin/bash
# Push the contract with WASM module to the network

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP_DIR="$SCRIPT_DIR/tmp"
CONTRACT_DIR="$TMP_DIR/wasm-contract"

echo "=== Pushing Contract to Network ==="
echo ""

# Check if contract exists
if [ ! -f "$TMP_DIR/contract_id.txt" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first"
    exit 1
fi

CONTRACT_ID=$(cat "$TMP_DIR/contract_id.txt")
echo "Contract ID: $CONTRACT_ID"
echo ""

# Get network address (assuming local devnet)
NETWORK_ADDR="/ip4/127.0.0.1/tcp/9001"
echo "Network address: $NETWORK_ADDR"
echo ""

# Push contract
echo "Pushing contract to network..."
modal contract push \
    "$CONTRACT_DIR" \
    --network "$NETWORK_ADDR" \
    --output json > "$TMP_DIR/push-output.json" || {
    echo "⚠️  Push failed (network may not be running)"
    echo "   You can still inspect the local contract"
    exit 0
}

cat "$TMP_DIR/push-output.json"
echo ""

echo "✓ Contract pushed to network"
echo ""
echo "The WASM module is now being validated by consensus nodes."
echo "Once confirmed, it will be available for validation."
echo ""

echo "=== Push Complete ==="
echo ""
echo "Next: Run ./04-test-validation.sh to test"
echo ""

