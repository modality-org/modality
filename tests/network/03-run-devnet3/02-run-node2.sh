#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node2 if it doesn't exist
if [ ! -f "./tmp/node2/config.json" ]; then
    echo "Creating node2 with standard devnet3/node2 identity..."

    # Create node using template with local bootstrappers for local devnet
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node2" \
        --from-template devnet3/node2 \
        --bootstrappers "/ip4/127.0.0.1/tcp/10301/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd,/ip4/127.0.0.1/tcp/10303/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
fi

modal node clear-storage --dir ./tmp/node2 --yes
modal node run-validator --dir ./tmp/node2