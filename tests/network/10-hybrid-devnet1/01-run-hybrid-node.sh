#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create hybrid node if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating hybrid node for devnet1-hybrid..."
    
    # Create basic node without template (we'll configure it manually)
    modal node create --dir "${SCRIPT_DIR}/tmp/node1"
    
    # Update config to use devnet1-hybrid network and enable hybrid consensus
    cat > "${SCRIPT_DIR}/tmp/node1/config.json" << 'EOF'
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10111/ws"],
  "network_config_path": "modal-networks://devnet1-hybrid",
  "run_miner": true,
  "hybrid_consensus": true,
  "run_validator": true,
  "initial_difficulty": 1,
  "status_port": 3111
}
EOF
fi

# Clear storage for clean test
modal node clear-storage --dir ./tmp/node1 --yes

# Run the hybrid node (both miner and validator)
echo "Starting hybrid node - will mine blocks and validate from epoch 2..."
modal node run-miner --dir ./tmp/node1

