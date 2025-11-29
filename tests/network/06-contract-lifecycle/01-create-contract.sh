#!/usr/bin/env bash
# Create a new contract locally
# This initializes a .contract directory with the contract metadata

set -e
cd "$(dirname "$0")"

# Setup
CONTRACT_DIR="./tmp/my-contract"

echo "📝 Creating a new contract..."
echo "=============================="
echo ""

# Clean up any previous contract
rm -rf "$CONTRACT_DIR"
mkdir -p "$CONTRACT_DIR"
cd "$CONTRACT_DIR"

# Create the contract
echo "Running: modal contract create --output json"
modal contract create --output json | tee create_result.json

# Extract contract ID
CONTRACT_ID=$(cat create_result.json | grep -o '"contract_id":"[^"]*"' | cut -d'"' -f4 || echo "")

echo ""
echo "✅ Contract created successfully!"
echo ""
echo "📋 Contract Details:"
echo "   Contract ID: $CONTRACT_ID"
echo "   Directory: $CONTRACT_DIR"
echo ""
echo "📁 Directory Structure:"
ls -la .contract/
echo ""
echo "💡 Tip: View contract config with: cat $CONTRACT_DIR/.contract/config.json"

