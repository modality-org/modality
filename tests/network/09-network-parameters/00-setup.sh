#!/usr/bin/env bash
set -e

echo "Setting up test environment..."

# Create temp directory for this test
mkdir -p ./tmp/test-network-params


# Create a node using devnet1 configuration (let it generate its own passfile)
echo "Creating test node with devnet1 configuration..."
modal node create \
    --node-id test-network-params-node \
    --storage-path ./tmp/test-network-params/storage \
    --dir ./tmp/test-network-params

# Update the config to use devnet1 network config file
CONFIG_FILE="./tmp/test-network-params/config.json"
TMP_FILE="./tmp/test-network-params/config.json.tmp"

# Use jq to add network_config_path
if command -v jq &> /dev/null; then
    NETWORK_CONFIG_ABS_PATH="$(cd ../../../fixtures/network-configs/devnet1 && pwd)/config.json"
    jq ".network_config_path = \"$NETWORK_CONFIG_ABS_PATH\"" "$CONFIG_FILE" > "$TMP_FILE"
    mv "$TMP_FILE" "$CONFIG_FILE"
fi

echo "✓ Test environment setup complete"
echo "  Storage: ./tmp/test-network-params/storage"
echo "  Config: ./tmp/test-network-params/config.json"
echo "  Passfile: ./tmp/test-network-params/node.passfile"
echo "  Network: devnet1 (with genesis contract)"





