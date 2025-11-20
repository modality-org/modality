#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create miner2 if it doesn't exist
if [ ! -f "./tmp/miner2/config.json" ]; then
    echo "Creating miner2..."
    
    modal node create --dir "${SCRIPT_DIR}/tmp/miner2"
    
    # Get miner1's peer ID
    MINER1_PEER_ID=$(jq -r '.id' ./tmp/miner1/config.json)
    
    # Configure as miner connected to miner1
    # Also with very low difficulty to race with miner1
    cat > "${SCRIPT_DIR}/tmp/miner2/config.json" << EOF
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10402/ws"],
  "bootstrappers": [
    "/ip4/127.0.0.1/tcp/10401/ws/p2p/${MINER1_PEER_ID}"
  ],
  "run_miner": true,
  "miner_nominees": [
    "12D3KooWBxABXy8BbxT5vqKmXPz1uy6GqHhQxmZ4hFxCLm6zGoFM"
  ],
  "initial_difficulty": 1,
  "status_port": 8402
}
EOF
fi

# Clear storage for clean test
modal node clear-storage --dir ./tmp/miner2 --yes

# Run the miner
echo "Starting miner2 (connected to miner1)..."
RUST_LOG=info modal node run-miner --dir ./tmp/miner2

