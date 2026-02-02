---
sidebar_position: 2
title: Multisig Treasury
---

# Building a Multisig Treasury Contract

Learn to create a 2-of-3 multisig treasury using the `threshold` predicate.

## What We're Building

A treasury contract where:
- 3 keyholders control the funds
- Any 2 can approve withdrawals
- All 3 required to change keyholders

## Step 1: Create Identities

```bash
# Create keyholder identities
modal id create --name alice
modal id create --name bob
modal id create --name carol
```

## Step 2: Create the Contract

```bash
mkdir treasury && cd treasury
modal contract create
modal c checkout
```

## Step 3: Set Up State

```bash
# Add keyholder identities
modal c set-named-id /treasury/alice.id --named alice
modal c set-named-id /treasury/bob.id --named bob
modal c set-named-id /treasury/carol.id --named carol

# Create signers list
mkdir -p state/treasury
echo '["/treasury/alice.id", "/treasury/bob.id", "/treasury/carol.id"]' \
  > state/treasury/signers.json
```

## Step 4: Define the Rules

Create `rules/treasury-auth.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // All commits must be signed by a keyholder
    signed_by(/treasury/alice.id) | signed_by(/treasury/bob.id) | signed_by(/treasury/carol.id)
  }
}
```

Create `rules/treasury-threshold.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // Withdrawals require 2-of-3
    always([+WITHDRAW] implies <+threshold(2, /treasury/signers.json)> true)
  }
}
```

## Step 5: Synthesize the Model

Use the multisig template:

```bash
modality model synthesize --template multisig -o model/treasury.modality
```

Or synthesize from your rules:

```bash
modality model synthesize --rule rules/treasury-auth.modality -o model/treasury.modality
```

The generated model enforces your threshold requirements:

```modality
export default model {
  initial locked
  
  // Propose withdrawal (any keyholder)
  locked -> pending [+signed_by(/treasury/alice.id)]
  locked -> pending [+signed_by(/treasury/bob.id)]
  locked -> pending [+signed_by(/treasury/carol.id)]
  
  // Execute withdrawal (2-of-3)
  pending -> executed [+threshold(2, /treasury/signers.json)]
  
  // Reset after execution
  executed -> locked [+signed_by(/treasury/alice.id)]
  executed -> locked [+signed_by(/treasury/bob.id)]
  executed -> locked [+signed_by(/treasury/carol.id)]
}
```

## Step 6: Commit and Test

```bash
modal c commit --all --sign alice -m "Initialize treasury"
```

### Propose a Withdrawal

```bash
echo '{"amount": 100, "to": "recipient_address"}' > state/treasury/proposal.json
modal c commit --all --sign alice -m "Alice proposes withdrawal"
```

### First Approval (Bob)

```bash
modal c commit --all --sign bob -m "Bob approves"
```

### Second Approval & Execute (Carol)

With 2 signatures collected, the withdrawal can execute:

```bash
modal c commit --all --sign carol -m "Execute withdrawal"
```

## How Threshold Works

The `threshold(n, signers_path)` predicate:

1. Loads the signer list from the path
2. Collects signatures from the commit
3. Verifies each signature is from an authorized signer
4. Ensures at least `n` unique valid signatures exist

**Key features:**
- Can't use the same signer twice
- Rejects unauthorized signers
- Works with any n-of-m configuration

## Available Templates

List all synthesis templates:

```bash
modality model synthesize --list
```

Templates include: `escrow`, `handshake`, `mutual_cooperation`, `atomic_swap`, `multisig`, `service_agreement`, `delegation`, `auction`, `subscription`, `milestone`.
