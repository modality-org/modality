#!/usr/bin/env bash
cd $(dirname -- "$0")
SCRIPT_DIR=$(pwd)
set -x

# Create miner1 if it doesn't exist
if [ ! -f "./tmp/node1/config.json" ]; then
    echo "Creating miner1 for devnet3-hybrid..."
    
    modal node create --dir "${SCRIPT_DIR}/tmp/node1"
    
    # Configure as hybrid miner/validator
    # This node will nominate all 3 nodes in rotation
    cat > "${SCRIPT_DIR}/tmp/node1/config.json" << 'EOF'
{
  "passfile_path": "./node.passfile",
  "storage_path": "./storage",
  "listeners": ["/ip4/0.0.0.0/tcp/10311/ws"],
  "bootstrappers": [
    "/ip4/127.0.0.1/tcp/10312/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "/ip4/127.0.0.1/tcp/10313/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
  ],
  "network_config_path": "modal-networks://devnet3-hybrid",
  "run_miner": true,
  "miner_nominees": [
    "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
  ],
  "hybrid_consensus": true,
  "run_validator": true,
  "initial_difficulty": 1,
  "status_port": 3311
}
EOF
fi

# Clear storage for clean test
modal node clear-storage --dir ./tmp/node1 --yes

# Run the hybrid node
echo "Starting miner1 (hybrid node)..."
modal node run-miner --dir ./tmp/node1

