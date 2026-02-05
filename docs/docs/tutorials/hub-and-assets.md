# Hub and Contract Assets Tutorial

This tutorial walks through running a contract hub and managing assets between contracts.

## Prerequisites

Install the Modal CLI:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://www.modality.org/install.sh | sh
```

## Starting a Hub

A hub is a centralized server that validates and stores contract commits. Start one locally:

```bash
# Start hub on port 3000
modal hub start --port 3000 --data-dir ./hub-data

# Output:
# Starting Modality Hub
#   Data directory: ./hub-data
#   Listening on: 0.0.0.0:3000
# Hub ready - accepting connections
# RPC endpoints:
#   POST http://0.0.0.0:3000/          (JSON-RPC)
#   GET  http://0.0.0.0:3000/health    (Health check)
#   WS   ws://0.0.0.0:3000/ws          (WebSocket)
```

The hub validates:
- **CREATE**: Asset doesn't already exist, valid quantity/divisibility
- **SEND**: Asset exists, sender has sufficient balance, amount respects divisibility
- **RECV**: Matching SEND exists, recipient matches, not already received
- **REPOST**: Value matches source contract's latest state

## Creating Contracts

Create two contracts that will exchange assets:

```bash
# Create Alice's contract
mkdir alice-contract && cd alice-contract
modal contract create --id alice_contract_001

# Create Bob's contract  
cd .. && mkdir bob-contract && cd bob-contract
modal contract create --id bob_contract_001
```

## Asset Lifecycle

### 1. CREATE - Mint a New Asset

Alice creates a token with 1,000,000 units (divisible by 100):

```bash
cd alice-contract

# Create the asset
modal contract commit \
  --method create \
  --asset-id ALICE_TOKEN \
  --quantity 1000000 \
  --divisibility 100

# Output:
# ✅ Commit created successfully!
#    Contract ID: alice_contract_001
#    Commit ID: a1b2c3d4...
```

View the created asset:

```bash
modal contract assets list

# Output:
# Assets in contract alice_contract_001:
#   - ALICE_TOKEN
#     Quantity: 1000000
#     Divisibility: 100
```

### 2. SEND - Transfer Assets

Alice sends 50,000 tokens to Bob:

```bash
# In alice-contract directory
modal contract commit \
  --method send \
  --asset-id ALICE_TOKEN \
  --to-contract bob_contract_001 \
  --amount 50000

# Output:
# ✅ Commit created successfully!
#    Contract ID: alice_contract_001
#    Commit ID: e5f6g7h8...
```

Note the commit ID (`e5f6g7h8...`) — Bob needs this to receive the tokens.

### 3. RECV - Receive Assets

Bob receives the tokens by referencing Alice's SEND commit:

```bash
cd ../bob-contract

# Receive the tokens
modal contract commit \
  --method recv \
  --send-commit-id e5f6g7h8...

# Output:
# ✅ Commit created successfully!
#    Contract ID: bob_contract_001
#    Commit ID: i9j0k1l2...
```

### 4. Check Balances

```bash
# Alice's balance (in alice-contract)
modal contract assets balance --asset-id ALICE_TOKEN
# Balance: 950000

# Bob's balance (in bob-contract)  
modal contract assets balance --asset-id ALICE_TOKEN --owner alice_contract_001
# Balance: 50000
```

## Pushing to Hub

After creating commits locally, push them to the hub for validation:

```bash
# Add hub as remote (in each contract directory)
modal contract remote add origin http://localhost:3000

# Push commits
modal contract push

# Output:
# Pushing to http://localhost:3000...
# ✅ Pushed 2 commits
#    New HEAD: e5f6g7h8...
```

The hub will reject invalid commits:

```bash
# Try to send more than you have
modal contract commit \
  --method send \
  --asset-id ALICE_TOKEN \
  --to-contract bob_contract_001 \
  --amount 999999999

modal contract push
# Error: SEND rejected: Insufficient balance: have 950000, need 999999999
```

## Complete Example: Token Exchange

Here's a full example of two contracts exchanging different tokens:

```bash
#!/bin/bash
set -e

# Start hub in background
modal hub start --port 3000 --data-dir ./hub-data &
HUB_PID=$!
sleep 2

# Create Alice's contract with ALPHA tokens
mkdir -p alice && cd alice
modal contract create --id alice_001
modal contract remote add origin http://localhost:3000
modal contract commit --method create --asset-id ALPHA --quantity 1000 --divisibility 1
modal contract push
ALICE_HEAD=$(modal contract commit-id)
cd ..

# Create Bob's contract with BETA tokens
mkdir -p bob && cd bob
modal contract create --id bob_001
modal contract remote add origin http://localhost:3000
modal contract commit --method create --asset-id BETA --quantity 500 --divisibility 1
modal contract push
cd ..

# Alice sends 100 ALPHA to Bob
cd alice
modal contract commit --method send --asset-id ALPHA --to-contract bob_001 --amount 100
modal contract push
SEND_ALPHA=$(modal contract commit-id)
cd ..

# Bob receives ALPHA and sends 50 BETA to Alice
cd bob
modal contract commit --method recv --send-commit-id $SEND_ALPHA
modal contract commit --method send --asset-id BETA --to-contract alice_001 --amount 50
modal contract push
SEND_BETA=$(modal contract commit-id)
cd ..

# Alice receives BETA
cd alice
modal contract commit --method recv --send-commit-id $SEND_BETA
modal contract push
cd ..

echo "Exchange complete!"
echo "Alice: 900 ALPHA, 50 BETA"
echo "Bob: 100 ALPHA, 450 BETA"

# Cleanup
kill $HUB_PID
```

## Asset Rules

| Property | Description | Validation |
|----------|-------------|------------|
| `asset_id` | Unique identifier within contract | Cannot duplicate |
| `quantity` | Total supply | Must be > 0 |
| `divisibility` | Smallest unit | Must be > 0, amounts must be divisible |

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| -32020 | Zero amount | SEND amount must be > 0 |
| -32021 | Sender not found | Contract doesn't exist |
| -32022 | Asset not found | Asset doesn't exist in contract |
| -32023 | Divisibility | Amount not divisible by asset divisibility |
| -32024 | Insufficient balance | Not enough tokens to send |
| -32030 | SEND not found | Referenced SEND commit doesn't exist |
| -32031 | Wrong recipient | RECV contract doesn't match SEND's to_contract |
| -32032 | Already received | SEND already received (double-spend prevention) |
| -32040 | Asset exists | CREATE failed, asset already exists |

## Next Steps

- [Commit Methods Reference](/docs/reference/commit-methods) - All commit types
- [Modal Logic](/docs/concepts/modal-logic) - Add rules to protect assets
- [Cross-Contract Data](/docs/reference/commit-methods#repost) - REPOST for data sharing
