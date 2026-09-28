#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 5: Bob Receives Tokens"
echo "================================================"
echo ""

cd tmp/bob

BOB_CONTRACT_ID=$(modal contract id)
SEND_COMMIT_ID=$(cat ../send-commit-id.txt)

echo "Bob receiving tokens from Alice..."
echo "  Send Commit ID: $SEND_COMMIT_ID"
echo ""

modal contract commit \
  --method recv \
  --send-commit-id "$SEND_COMMIT_ID"

RECV_COMMIT_ID=$(modal contract commit-id)

echo "✅ RECV action created!"
echo ""
echo "Recv Commit ID: $RECV_COMMIT_ID"
echo ""

echo "💾 Pushing commits to sequencer..."
echo ""

# Only push if SKIP_PUSH is not set (for network tests)
if [ -z "$SKIP_PUSH" ]; then
    # Push to sequencer (note: /ws for WebSocket protocol)
    # Don't use --node-dir to avoid peer ID conflict with sequencer
    modal contract push \
      --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
      --remote-name origin

    echo ""
    echo "✅ Commits pushed to network!"
    echo ""
    echo "Bob's balance will be updated through network consensus"
else
    echo "(Skipped - local mode)"
    echo ""
    echo "Bob's balance is tracked locally"
fi
echo ""

cd ../..

