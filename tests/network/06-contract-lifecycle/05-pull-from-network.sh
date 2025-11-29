#!/usr/bin/env bash
# Pull commits from the network to local contract
# Demonstrates fetching remote changes

set -e
cd "$(dirname "$0")"

CONTRACT_DIR="./tmp/my-contract"
CONTRACT_DIR2="./tmp/my-contract-clone"

echo "📥 Pulling commits from network..."
echo "=================================="
echo ""

# For this demo, we'll simulate pulling by creating a second contract instance
# In a real scenario, this would pull from validators

# Ensure original contract exists
if [ ! -d "$CONTRACT_DIR/.contract" ]; then
    echo "❌ Error: Original contract not found. Run the previous examples first."
    exit 1
fi

# Create a second contract directory to demonstrate pull
echo "Setting up a second contract instance to demonstrate pull..."
rm -rf "$CONTRACT_DIR2"
mkdir -p "$CONTRACT_DIR2"

# Copy contract ID and config to simulate a clone
CONTRACT_ID=$(cat "$CONTRACT_DIR/.contract/config.json" | grep -o '"contract_id":"[^"]*"' | cut -d'"' -f4 || echo "")
echo "   Contract ID: $CONTRACT_ID"
echo ""

cd "$CONTRACT_DIR2"

# Initialize with same contract ID (simulating a clone)
mkdir -p .contract
cp "$CONTRACT_DIR/.contract/config.json" .contract/
cp "$CONTRACT_DIR/.contract/genesis.json" .contract/ 2>/dev/null || true
cp "$CONTRACT_DIR/.contract/HEAD" .contract/ 2>/dev/null || true

echo "Pulling commits from network..."
modal contract pull --output json 2>&1 | tee pull_result.json || true
echo ""

# Check result
if [ -f pull_result.json ] && (grep -q '"pulled":' pull_result.json 2>/dev/null || grep -q '"success"' pull_result.json 2>/dev/null); then
    PULLED_COUNT=$(cat pull_result.json | grep -o '"pulled":[0-9]*' | grep -o '[0-9]*' || echo "?")
    echo "✅ Pull completed!"
    echo ""
    echo "📊 Pull Summary:"
    echo "   Commits pulled: $PULLED_COUNT"
else
    echo "⚠️  Pull completed (check output for details)"
    echo ""
    echo "💡 Note: This example demonstrates the pull command."
    echo "         In a real network, validators would serve the commits."
fi

echo ""
echo "💡 Tip: View pulled changes with: modal contract status"

