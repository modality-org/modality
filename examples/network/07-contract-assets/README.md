# Contract Assets Example (with devnet1)

This example demonstrates how to create, send, and receive assets within contracts using a real network validator (devnet1).

## Overview

This example shows the complete lifecycle of contract assets with network consensus:
1. Start a devnet1 validator node
2. Create two contracts (Alice and Bob)
3. Alice creates a token asset
4. Alice sends tokens to Bob (pushed to network)
5. Bob receives the tokens (pushed to network)
6. Query balances at each step

## Prerequisites

- Built `modal` CLI: `cd rust && cargo build --bin modal`
- Ensure you're in the `examples/` directory or have loaded `.envrc` with direnv
- The scripts expect `modal` to be in your PATH (via debug build at `../rust/target/debug/modal`)

## Steps

### Step 0: Setup devnet1

Clean any previous state and create directories:

```bash
./00-setup-devnet1.sh
```

### Step 0.5: Start devnet1 Validator

Start the validator node in the background:

```bash
./00b-start-validator.sh
```

This will:
- Create a validator node from the devnet1/node1 template
- Start it on port 10101
- Run in the background with logs in `tmp/test-logs/validator.log`

### Step 1: Create Alice's Contract

Create the first contract (Alice):

```bash
./01-create-alice.sh
```

### Step 2: Create Token Asset

Alice creates a fungible token with 1,000,000 units and divisibility of 100:

```bash
./02-create-token.sh
```

### Step 3: Create Bob's Contract

Create the second contract (Bob):

```bash
./03-create-bob.sh
```

### Step 4: Alice Sends Tokens to Bob

Alice sends 10,000 tokens to Bob's contract and pushes to the network:

```bash
./04-alice-sends-tokens.sh
```

This will:
- Create a SEND action
- Push all commits (CREATE + SEND) to the validator
- The validator processes the commits through consensus

### Step 5: Bob Receives Tokens

Bob creates a RECV action to accept the tokens from Alice's SEND and pushes to the network:

```bash
./05-bob-receives-tokens.sh
```

This will:
- Create a RECV action referencing Alice's SEND
- Push the RECV commit to the validator
- The validator validates and processes the transfer through consensus

### Step 6: Query Balances

View the asset state in both contracts:

```bash
./06-query-balances.sh
```

### Step 7: Stop Validator

Clean shutdown of the validator node:

```bash
./07-stop-validator.sh
```

### Step 8: Invalid Double-Send Example

Demonstrates validator rejection of insufficient balance:

```bash
./08-invalid-double-send.sh
```

**What it does**:
- Attempts to send 1,500,000 tokens when Alice only has ~990,000
- Shows that validators reject the SEND at consensus level
- Demonstrates balance validation and double-spend prevention

**Expected result**:
- Local commit may be created
- Validator rejects with: `"Insufficient balance: have 990000, need 1500000"`
- Asset balances remain unchanged

## Running the Full Test

### Local Mode

Run the complete test suite locally (fast, no network needed):

```bash
./test.sh
```

**Status**: ✅ All 27 tests pass

This demonstrates:
- Asset creation, sending, and receiving
- Local validation and balance tracking
- Commit structure verification
- Invalid operation handling (insufficient balance)

### Network Mode (devnet1)

Run with a real validator node and network consensus:

```bash
./test-devnet1.sh
```

**Status**: ✅ All 18 tests pass

This demonstrates:
- Real network consensus with validator
- Push/pull workflow for commits
- Asset state tracking through consensus

This will:
- Clean previous state
- Start devnet1 validator
- Run all steps in sequence
- Push commits to the network
- Validate output at each step
- Stop the validator
- Report success or failure

## Network Features

This example demonstrates:
- **Real network consensus**: Commits are processed by a validator node
- **Push/Pull workflow**: Like git, contracts push commits to validators
- **Consensus validation**: The validator validates and orders commits
- **Asset state tracking**: Balances are updated through network consensus

## Local vs Network

- **Local mode** (`./test.sh`): Fast testing without network
- **Network mode** (`./test-devnet1.sh`): Full consensus with devnet1

## Validator Details

- **Network**: devnet1 (single validator)
- **Port**: 10101
- **Peer ID**: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`
- **Template**: `devnet1/node1`

## Validation Examples

The example includes demonstrations of consensus-level validation:

### Valid Operations
- **CREATE**: Alice creates 1,000,000 tokens with divisibility 100
- **SEND**: Alice sends 10,000 tokens to Bob (has sufficient balance)
- **RECV**: Bob receives the tokens (is the intended recipient)

### Invalid Operations (Rejected by Validators)

#### Invalid SEND - Insufficient Balance
```bash
./08-invalid-double-send.sh
```

Demonstrates what happens when trying to send more than you have:
- Alice tries to send 1,500,000 tokens
- Alice only has 990,000 tokens remaining
- **Result**: Validator rejects with `"Insufficient balance: have 990000, need 1500000"`

This prevents:
- Double-spending attacks
- Creating assets from nothing
- Balance going negative

#### Invalid RECV - Wrong Recipient
If a contract tries to receive a SEND intended for someone else:
- **Result**: Validator rejects with `"RECV rejected: not the intended recipient"`

#### Invalid RECV - Double Receive
If a contract tries to receive the same SEND twice:
- **Result**: Validator rejects with `"SEND already received by contract X"`

## Asset Types

This example demonstrates fungible tokens, but you can create different asset types:

### Native Token (like Bitcoin)
```bash
modal contract commit --method create \
  --asset-id btc \
  --quantity 21000000 \
  --divisibility 100000000
```

### Non-Fungible Token (NFT)
```bash
modal contract commit --method create \
  --asset-id rare_art_001 \
  --quantity 1 \
  --divisibility 1
```

### Custom Token
```bash
modal contract commit --method create \
  --asset-id my_token \
  --quantity 1000000 \
  --divisibility 100
```

## Directory Structure

```
07-contract-assets/
├── README.md
├── 00-setup.sh
├── 01-create-alice.sh
├── 02-create-token.sh
├── 03-create-bob.sh
├── 04-alice-sends-tokens.sh
├── 05-bob-receives-tokens.sh
├── 06-query-balances.sh
├── test.sh
└── data/
    ├── alice/
    └── bob/
```

## Expected Output

After running all steps, you should see:
- Alice's balance: 990,000 tokens (1,000,000 - 10,000)
- Bob's balance: 10,000 tokens (received from Alice)

## Cleanup

To clean up and start over:

```bash
./00-setup.sh
```

## Notes

- This example uses local contracts only (no network interaction)
- For network testing, use `modal contract push` to push commits to validators
- Asset state is tracked locally until pushed to the network
- RECV actions reference SEND commits by their commit ID

