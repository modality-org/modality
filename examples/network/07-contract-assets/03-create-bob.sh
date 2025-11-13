#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 3: Create Bob's Contract"
echo "================================================"
echo ""

cd data/bob

echo "Creating Bob's contract..."
modal contract create --output json > bob-contract.json

BOB_CONTRACT_ID=$(cat bob-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
BOB_GENESIS_COMMIT=$(cat bob-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['genesis_commit_id'])")

echo "✅ Bob's contract created!"
echo ""
echo "Contract ID: $BOB_CONTRACT_ID"
echo "Genesis Commit: $BOB_GENESIS_COMMIT"
echo ""
echo "Contract directory: data/bob/.contract/"
echo ""

cd ../..

