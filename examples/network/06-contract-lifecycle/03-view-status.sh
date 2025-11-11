#!/usr/bin/env bash
# View the status of a local contract
# Shows local commits and remote status

set -e
cd "$(dirname "$0")"

CONTRACT_DIR="./tmp/my-contract"

echo "📊 Viewing contract status..."
echo "=============================="
echo ""

# Ensure contract exists
if [ ! -d "$CONTRACT_DIR/.contract" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first."
    exit 1
fi

cd "$CONTRACT_DIR"

echo "Human-readable status:"
echo "----------------------"
modal contract status
echo ""

echo "JSON status:"
echo "------------"
modal contract status --output json | tee status.json
echo ""

# Parse and display key info
CONTRACT_ID=$(cat status.json | grep -o '"contract_id":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
LOCAL_COMMITS=$(cat status.json | grep -o '"local_commits":[0-9]*' | grep -o '[0-9]*' || echo "0")

echo "✅ Status retrieved successfully!"
echo ""
echo "📋 Summary:"
echo "   Contract ID: $CONTRACT_ID"
echo "   Local commits: $LOCAL_COMMITS"
echo ""
echo "💡 Tip: Push commits to validators with: modal contract push"

