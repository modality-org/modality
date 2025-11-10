#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating node1 with standard devnet3/node1 identity..."

    # Create node using template
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node1" \
        --from-template devnet3/node1
fi

modal node clear-storage --dir ./tmp/node1 --yes
modal node run-validator --dir ./tmp/node1