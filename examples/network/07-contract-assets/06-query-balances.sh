#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 6: Query Balances"
echo "================================================"
echo ""

echo "--- Alice's Assets ---"
cd data/alice
ALICE_CONTRACT_ID=$(cat alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
echo "Contract ID: $ALICE_CONTRACT_ID"
echo ""

modal contract assets list
echo ""
modal contract assets balance --asset-id my_token
echo ""

cd ../..

echo "--- Bob's Assets ---"
cd data/bob
BOB_CONTRACT_ID=$(cat bob-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
echo "Contract ID: $BOB_CONTRACT_ID"
echo ""

modal contract assets list
echo ""
echo "Note: Bob's assets will show once the RECV is processed by the network"
echo ""

cd ../..

echo "================================================"
echo "Summary"
echo "================================================"
echo ""
echo "✅ Alice created 1,000,000 tokens"
echo "✅ Alice sent 10,000 tokens to Bob"
echo "✅ Bob created RECV action"
echo ""
echo "Local tracking shows Alice has ~990,000 tokens"
echo "Full balance updates require network consensus processing"
echo ""

