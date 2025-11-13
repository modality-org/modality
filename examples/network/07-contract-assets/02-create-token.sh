#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 2: Create Token Asset"
echo "================================================"
echo ""

cd data/alice

ALICE_CONTRACT_ID=$(cat alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")

echo "Creating fungible token asset..."
echo "  Asset ID: my_token"
echo "  Quantity: 1,000,000"
echo "  Divisibility: 100"
echo ""

modal contract commit \
  --method create \
  --asset-id my_token \
  --quantity 1000000 \
  --divisibility 100 \
  --output json > create-token.json

CREATE_COMMIT_ID=$(cat create-token.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")

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

