#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 5: Bob Receives Tokens"
echo "================================================"
echo ""

cd data/bob

BOB_CONTRACT_ID=$(cat bob-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
SEND_COMMIT_ID=$(cat ../send-commit-id.txt)

echo "Bob receiving tokens from Alice..."
echo "  Send Commit ID: $SEND_COMMIT_ID"
echo ""

modal contract commit \
  --method recv \
  --send-commit-id "$SEND_COMMIT_ID" \
  --output json > recv-tokens.json

RECV_COMMIT_ID=$(cat recv-tokens.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")

echo "✅ RECV action created!"
echo ""
echo "Recv Commit ID: $RECV_COMMIT_ID"
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
echo "Bob's balance will be updated through network consensus"
echo ""

cd ../..

