#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create miner1 if it doesn't exist
if [ ! -f "./tmp/miner1/config.json" ]; then
    echo "Creating miner1..."
    
    modal node create --dir "${SCRIPT_DIR}/tmp/miner1"
    
    # Configure as miner with very low difficulty to increase mining speed
    # This increases the likelihood of race condition
    cat > "${SCRIPT_DIR}/tmp/miner1/config.json" << 'EOF'
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10401/ws"],
  "bootstrappers": [],
  "run_miner": true,
  "miner_nominees": [
    "12D3KooWBxABXy8BbxT5vqKmXPz1uy6GqHhQxmZ4hFxCLm6zGoFM"
  ],
  "initial_difficulty": 1,
  "status_port": 8401
}
EOF
fi

# Clear storage for clean test
modal node clear-storage --dir ./tmp/miner1 --yes

# Run the miner
echo "Starting miner1..."
RUST_LOG=info modal node run-miner --dir ./tmp/miner1

