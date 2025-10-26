#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Sync blocks from a specific epoch
# Requires node1 to be running (see 01-run-node1.sh)

NODE1_PEER_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
EPOCH=${1:-0}

echo "Syncing miner blocks from epoch $EPOCH..."

modal net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/$NODE1_PEER_ID \
  --mode epoch \
  --epoch $EPOCH \
  --persist

echo ""
echo "Epoch $EPOCH sync completed!"

