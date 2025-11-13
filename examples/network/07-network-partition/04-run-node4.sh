#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)

# Note: devnet3 only has 3 standard nodes, so we create node4 dynamically
if [ ! -f "./tmp/node4/config.json" ]; then
    echo "Creating node4..."
    modal node create --dir ./tmp/node4 --network devnet3
fi

modal node clear-storage --dir ./tmp/node4 --yes
modal node run-validator --dir ./tmp/node4

