#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node3 if it doesn't exist
if [ ! -f "./tmp/node3/config.json" ]; then
    echo "Creating node3 with standard devnet3/node3 identity..."
    
    # Create node using template with local bootstrappers for local devnet
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node3" \
        --from-template devnet3/node3 \
        --bootstrappers "/ip4/127.0.0.1/tcp/10301/ws/p2p/12D3KooWA7csjq5MQyPWA4R7R5jRMf8RwhnSjXNNY3fF2jH2uxK3,/ip4/127.0.0.1/tcp/10302/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB"
fi

modal node clear-storage --dir ./tmp/node3 --yes
modal node run-validator --dir ./tmp/node3

