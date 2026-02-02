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

## Step 5: Define the Model

Create `model/treasury.modality` — the state machine satisfying the rules:

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

## Advanced: Graduated Thresholds

```modality
export default model {
  initial active
  
  // Low-value: 1-of-3
  active -> active [+threshold(1, /treasury/signers.json)]
  
  // High-value: 2-of-3  
  active -> pending_large [+signed_by(/treasury/alice.id)]
  active -> pending_large [+signed_by(/treasury/bob.id)]
  active -> pending_large [+signed_by(/treasury/carol.id)]
  pending_large -> active [+threshold(2, /treasury/signers.json)]
  
  // Add/remove signer: 3-of-3 (unanimous)
  active -> active [+threshold(3, /treasury/signers.json)]
}
```
