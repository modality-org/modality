#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node2 if it doesn't exist
if [ ! -f "./tmp/node2/config.json" ]; then
    echo "Creating node2 with standard devnet2/node2 identity..."

    # Create node using template
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node2" \
        --from-template devnet2/node2
fi

cd "./tmp/node2"
modal node clear-storage --yes
modal node run --enable-consensus