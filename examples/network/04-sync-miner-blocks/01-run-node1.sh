#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Setup node1 with test blocks if not already done
if [ ! -d "./tmp/storage/node1" ] || [ ! "$(ls -A ./tmp/storage/node1 2>/dev/null)" ]; then
    echo "Node1 datastore is empty. Setting up test blocks..."
    ./00-setup-node1-blocks.sh
    echo ""
fi

# Run node1 which will have miner blocks
echo "Starting node1 with miner blocks..."
modality net run-node --config ./configs/node1.json

