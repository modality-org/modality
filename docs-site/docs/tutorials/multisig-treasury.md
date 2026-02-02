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

## Step 1: Create the Contract

```bash
mkdir treasury && cd treasury
modal contract create

# Create keyholder identities
modal id create --path alice.passfile
modal id create --path bob.passfile
modal id create --path carol.passfile
```

## Step 2: Set Up State

```bash
modal c checkout
mkdir -p state/treasury

# Add keyholder identities
modal c set /treasury/alice.id $(modal id get --path ./alice.passfile)
modal c set /treasury/bob.id $(modal id get --path ./bob.passfile)
modal c set /treasury/carol.id $(modal id get --path ./carol.passfile)

# Create signers list
echo '{"signers": ["/treasury/alice.id", "/treasury/bob.id", "/treasury/carol.id"]}' \
  > state/treasury/config.json
```

## Step 3: Define the Model

Create `model/treasury.modality`:

```modality
model treasury {
  initial locked
  
  // Propose withdrawal (any keyholder)
  locked -> pending [+PROPOSE +signed_by(/treasury/alice.id)]
  locked -> pending [+PROPOSE +signed_by(/treasury/bob.id)]
  locked -> pending [+PROPOSE +signed_by(/treasury/carol.id)]
  
  // Execute withdrawal (2-of-3)
  pending -> executed [+EXECUTE +threshold(2, /treasury/signers)]
  
  // Reset after execution
  executed -> locked [+RESET]
}
```

## Step 4: Test the Flow

### Propose a Withdrawal

```bash
modal c act PROPOSE --sign alice.passfile
modal c commit --all --sign alice.passfile -m "Alice proposes withdrawal"
```

### First Approval (Bob)

```bash
modal c act APPROVE --sign bob.passfile
modal c commit --all --sign bob.passfile -m "Bob approves"
```

### Second Approval & Execute (Carol)

```bash
modal c act EXECUTE --sign carol.passfile
modal c commit --all --sign carol.passfile -m "Execute withdrawal"
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
model graduated_treasury {
  initial active
  
  // Low-value: 1-of-3
  active -> active [+SMALL_WITHDRAWAL +threshold(1, /treasury/signers)]
  
  // High-value: 2-of-3
  active -> active [+LARGE_WITHDRAWAL +threshold(2, /treasury/signers)]
  
  // Add/remove signer: 3-of-3 (unanimous)
  active -> active [+CHANGE_SIGNERS +threshold(3, /treasury/signers)]
}
```
