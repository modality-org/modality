#!/usr/bin/env bash
set -e

echo "Verifying genesis contract in network config..."

# Check that devnet1 config has genesis_contract_id
NETWORK_CONFIG="../../../fixtures/network-configs/devnet1/config.json"

if [ ! -f "$NETWORK_CONFIG" ]; then
    echo "Error: Network config not found: $NETWORK_CONFIG"
    exit 1
fi

# Extract genesis_contract_id
GENESIS_CONTRACT_ID=$(cat "$NETWORK_CONFIG" | python3 -c "import sys, json; print(json.load(sys.stdin).get('genesis_contract_id', ''))")

if [ -z "$GENESIS_CONTRACT_ID" ]; then
    echo "Error: No genesis_contract_id found in network config"
    exit 1
fi

echo "✓ Genesis contract ID: $GENESIS_CONTRACT_ID"

# Save for later steps
echo "$GENESIS_CONTRACT_ID" > ./tmp/test-network-params/genesis-contract-id.txt

# Verify round 0 has contract-commit event
echo "Checking round 0 for contract-commit events..."
ROUND_0=$(cat "$NETWORK_CONFIG" | python3 -c "
import sys, json
config = json.load(sys.stdin)
rounds = config.get('rounds', {})
round_0 = rounds.get('0', {})
# Get first peer's block
for peer_id, block in round_0.items():
    events = block.get('events', [])
    for event in events:
        if event.get('type') == 'contract-commit':
            print('found')
            sys.exit(0)
sys.exit(1)
")

if [ "$ROUND_0" != "found" ]; then
    echo "Error: No contract-commit event found in round 0"
    exit 1
fi

echo "✓ Round 0 contains contract-commit event"

