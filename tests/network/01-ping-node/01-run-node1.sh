#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating node1 with standard devnet1/node1 identity..."
    
    # Create node using template
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node1" \
        --from-template devnet1/node1    
fi

cd "./tmp/node1"
modal node clear-storage --yes
modal node run