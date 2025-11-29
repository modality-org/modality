#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "🔨 Starting miner node..."
echo ""
echo "This will mine blocks continuously and demonstrate:"
echo "- Mining with proper difficulty calculation"
echo "- Difficulty adjustment after each epoch (40 blocks)"
echo "- Persistent blockchain state"
echo ""

# Build modal CLI if needed
command -v modal &> /dev/null || rebuild

# Clean up old node directory if requested
if [ "$1" == "--clean" ]; then
    echo "Cleaning up old node directory..."
    rm -rf ./tmp/miner
    echo ""
fi

# Create the miner node directory if it doesn't exist
if [ ! -d "./tmp/miner" ]; then
    echo "Creating miner node directory..."
    modal node create \
        --dir ./tmp/miner \
        --network devnet1
    
    # Update config to enable mining and set status port
    CONFIG_FILE="./tmp/miner/config.json"
    TMP_FILE="./tmp/miner/config.json.tmp"
    
    # Use jq to add run_miner and status_port if available, otherwise use sed
    if command -v jq &> /dev/null; then
        jq '. + {run_miner: true, status_port: 8080, initial_difficulty: 1, listeners: ["/ip4/0.0.0.0/tcp/10301/ws"]}' "$CONFIG_FILE" > "$TMP_FILE"
        mv "$TMP_FILE" "$CONFIG_FILE"
    else
        # Fallback to manual editing - add fields before the last }
        sed -i.bak '/"bootstrappers"/a\
  ,"run_miner": true\
  ,"status_port": 8080\
  ,"initial_difficulty": 1\
  ,"listeners": ["/ip4/0.0.0.0/tcp/10301/ws"]
' "$CONFIG_FILE"
        rm -f "$CONFIG_FILE.bak"
    fi
    echo ""
fi

echo "Node directory: $(pwd)/tmp/miner"
echo ""
echo "Press Ctrl+C to stop mining"
echo ""

# Run the miner
export RUST_LOG=info,modality_network_node=info
modal node run-miner --dir ./tmp/miner


