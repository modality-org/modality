#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Create node2 if it doesn't exist
if [ ! -f "./tmp/node2/config.json" ]; then
    echo "Creating node2..."
    modal node create --dir ./tmp/node2 --network devnet1
fi

# Ping node1 from node2
# Using standard devnet1/node1 peer ID and port 10101
NODE1_PEER_ID="12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"

modal node ping \
  --dir ./tmp/node2 \
  --target /ip4/127.0.0.1/tcp/10101/ws/p2p/$NODE1_PEER_ID \
  --times 20