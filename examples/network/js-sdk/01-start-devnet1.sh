#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create node1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating node1 with standard devnet1/node1 identity..."
    
    # Create node using template - single validator
    modal node create \
        --dir "${SCRIPT_DIR}/tmp/node1" \
        --from-template devnet1/node1
fi

# Clear storage to start fresh
modal node clear-storage --dir ./tmp/node1 --yes

# Run validator in background
echo "Starting devnet1 node1..."
modal node run-validator --dir ./tmp/node1 > ./tmp/node1-output.log 2>&1 &
NODE_PID=$!

echo "Node1 PID: ${NODE_PID}"
echo "${NODE_PID}" > ./tmp/node1.pid

# Wait for node to be ready
echo "Waiting for node to be ready..."
sleep 3

# Check if node is running
if ps -p ${NODE_PID} > /dev/null; then
    echo "✓ Node1 is running (PID: ${NODE_PID})"
    echo "✓ Listening on /ip4/0.0.0.0/tcp/10101/ws"
    echo "✓ Peer ID: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
else
    echo "✗ Node1 failed to start"
    cat ./tmp/node1-output.log
    exit 1
fi

