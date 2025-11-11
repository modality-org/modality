#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 4: Alice Sends Tokens to Bob"
echo "================================================"
echo ""

cd data/alice

ALICE_CONTRACT_ID=$(cat alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
BOB_CONTRACT_ID=$(cat ../bob/bob-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")

echo "Alice sending tokens to Bob..."
echo "  Asset ID: my_token"
echo "  To Contract: $BOB_CONTRACT_ID"
echo "  Amount: 10,000"
echo ""

modal contract commit \
  --method send \
  --asset-id my_token \
  --to-contract "$BOB_CONTRACT_ID" \
  --amount 10000 \
  --output json > send-tokens.json

SEND_COMMIT_ID=$(cat send-tokens.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")

# Save the SEND commit ID for Bob to use
echo "$SEND_COMMIT_ID" > ../send-commit-id.txt

echo "✅ SEND action created!"
echo ""
echo "Send Commit ID: $SEND_COMMIT_ID"
echo ""
echo "Note: Tokens deducted locally, will be finalized when pushed to network"
echo ""

# Query Alice's balance
echo "Querying Alice's balance..."
modal contract assets balance --asset-id my_token

echo ""
echo "💾 Pushing commits to validator..."
echo ""

# Push to validator (note: /ws for WebSocket protocol)
# Don't use --node-dir to avoid peer ID conflict with validator
modal contract push \
  --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
  --remote-name origin

echo ""
echo "✅ Commits pushed to network!"
echo ""

cd ../..

