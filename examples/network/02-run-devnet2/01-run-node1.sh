#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating node1 with standard devnet2/node1 identity..."

    # Create node using template with local bootstrapper for local devnet
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node1" \
        --from-template devnet2/node1 \
        --bootstrappers "/ip4/127.0.0.1/tcp/10202/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB"
fi

modal node clear-storage --dir ./tmp/node1 --yes
modal node run-validator --dir ./tmp/node1