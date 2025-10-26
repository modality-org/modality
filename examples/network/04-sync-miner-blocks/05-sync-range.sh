#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Sync a range of blocks
# Requires node1 to be running (see 01-run-node1.sh)

NODE1_PEER_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
FROM_INDEX=${1:-0}
TO_INDEX=${2:-10}

echo "Syncing miner blocks from index $FROM_INDEX to $TO_INDEX..."

modal net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/$NODE1_PEER_ID \
  --mode range \
  --from-index $FROM_INDEX \
  --to-index $TO_INDEX \
  --persist

echo ""
echo "Range sync completed!"
echo "Synced blocks $FROM_INDEX to $TO_INDEX"

