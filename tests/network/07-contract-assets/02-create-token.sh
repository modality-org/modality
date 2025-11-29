#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 2: Create Token Asset"
echo "================================================"
echo ""

cd tmp/alice

ALICE_CONTRACT_ID=$(modal contract id)

echo "Creating fungible token asset..."
echo "  Asset ID: my_token"
echo "  Quantity: 1,000,000"
echo "  Divisibility: 100"
echo ""

modal contract commit \
  --method create \
  --asset-id my_token \
  --quantity 1000000 \
  --divisibility 100

CREATE_COMMIT_ID=$(modal contract commit-id)

echo "✅ Token asset created!"
echo ""
echo "Contract ID: $ALICE_CONTRACT_ID"
echo "Create Commit: $CREATE_COMMIT_ID"
echo ""
echo "Alice now has 1,000,000 tokens"
echo ""

# Query the asset
echo "Querying asset state..."
modal contract assets list

cd ../..

