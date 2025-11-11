#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 1: Create Alice's Contract"
echo "================================================"
echo ""

cd data/alice

echo "Creating Alice's contract..."
modal contract create --output json > alice-contract.json

ALICE_CONTRACT_ID=$(cat alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
ALICE_GENESIS_COMMIT=$(cat alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['genesis_commit_id'])")

echo "✅ Alice's contract created!"
echo ""
echo "Contract ID: $ALICE_CONTRACT_ID"
echo "Genesis Commit: $ALICE_GENESIS_COMMIT"
echo ""
echo "Contract directory: data/alice/.contract/"
echo ""

cd ../..

