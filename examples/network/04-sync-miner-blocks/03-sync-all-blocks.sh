#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Sync all canonical miner blocks from node1 to node2
# Requires node1 to be running (see 01-run-node1.sh)

NODE1_PEER_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"

echo "Syncing all canonical miner blocks from node1 to node2..."

modal net mining sync \
  --config ./configs/node2.json \
  --target /ip4/127.0.0.1/tcp/10201/ws/p2p/$NODE1_PEER_ID \
  --mode all \
  --persist

echo ""
echo "Sync completed!"
echo "Check the summary output above for details."

