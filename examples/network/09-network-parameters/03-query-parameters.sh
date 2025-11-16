#!/usr/bin/env bash
set -e

echo "Querying network parameters from datastore..."

GENESIS_CONTRACT_ID=$(cat ./tmp/test-network-params/genesis-contract-id.txt)

echo "Genesis Contract ID: $GENESIS_CONTRACT_ID"

# Use modal node inspect to query the datastore
echo ""
echo "Checking for /network/name..."
modal node inspect \
    --config ./tmp/test-network-params/config.json \
    datastore-get "/contracts/$GENESIS_CONTRACT_ID/network/name.text" \
    > ./tmp/test-network-params/name.txt 2>&1 || echo "(not found yet)"

echo ""
echo "Checking for /network/difficulty..."
modal node inspect \
    --config ./tmp/test-network-params/config.json \
    datastore-get "/contracts/$GENESIS_CONTRACT_ID/network/difficulty.number" \
    > ./tmp/test-network-params/difficulty.txt 2>&1 || echo "(not found yet)"

echo ""
echo "Checking for /network/validators/0..."
modal node inspect \
    --config ./tmp/test-network-params/config.json \
    datastore-get "/contracts/$GENESIS_CONTRACT_ID/network/validators/0.text" \
    > ./tmp/test-network-params/validator0.txt 2>&1 || echo "(not found yet)"

echo ""
echo "✓ Parameter query complete"
echo "  Results saved to ./tmp/test-network-params/*.txt"

