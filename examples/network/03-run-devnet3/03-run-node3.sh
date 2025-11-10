#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node3 if it doesn't exist
if [ ! -f "./tmp/node3/config.json" ]; then
    echo "Creating node3 with standard devnet3/node3 identity..."

    # Create node using template
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node3" \
        --from-template devnet3/node3
fi

cd "./tmp/node3"
modal node clear-storage --yes
modal node run