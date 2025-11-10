#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node2 if it doesn't exist
if [ ! -f "./tmp/node2/config.json" ]; then
    echo "Creating node2 with standard devnet2/node2 identity..."

    # Create node using template with local bootstrapper for local devnet
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node2" \
        --from-template devnet2/node2 \
        --bootstrappers "/ip4/127.0.0.1/tcp/10201/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
fi

modal node clear-storage --dir ./tmp/node2 --yes
modal node run-validator --dir ./tmp/node2