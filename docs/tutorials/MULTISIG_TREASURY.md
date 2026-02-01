# Building a Multisig Treasury Contract

Learn to create a 2-of-3 multisig treasury using the `threshold` predicate.

---

## What We're Building

A treasury contract where:
- 3 keyholders control the funds
- Any 2 can approve withdrawals
- All 3 required to change keyholders

---

## Step 1: Create the Contract

```bash
mkdir treasury && cd treasury
modal contract create

# Create keyholder identities
modal id create --path alice.passfile
modal id create --path bob.passfile
modal id create --path carol.passfile
```

---

## Step 2: Set Up State

```bash
modal c checkout
mkdir -p state/treasury

# Add keyholder identities
modal c set /treasury/alice.id $(modal id get --path ./alice.passfile)
modal c set /treasury/bob.id $(modal id get --path ./bob.passfile)
modal c set /treasury/carol.id $(modal id get --path ./carol.passfile)

# Create signers list (JSON array of paths)
echo '{"signers": ["/treasury/alice.id", "/treasury/bob.id", "/treasury/carol.id"]}' > state/treasury/config.json
```

---

## Step 3: Define the Model

Create **model/treasury.modality**:

```modality
model treasury {
  initial locked
  
  // Propose withdrawal (any keyholder)
  locked -> pending [+PROPOSE +signed_by(/treasury/alice.id)]
  locked -> pending [+PROPOSE +signed_by(/treasury/bob.id)]
  locked -> pending [+PROPOSE +signed_by(/treasury/carol.id)]
  
  // Cancel (proposer only - tracked in state)
  pending -> locked [+CANCEL +signed_by(/treasury/proposer.id)]
  
  // Execute withdrawal (2-of-3)
  pending -> executed [+EXECUTE +threshold(2, /treasury/signers)]
  
  // Reset after execution
  executed -> locked [+RESET]
  
  // Terminal self-loop
  executed -> executed [+DONE]
}
```

---

## Step 4: Define the Rules

Create **rules/withdrawal.modality**:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // Withdrawals require 2-of-3 approval
    always ([+EXECUTE] implies threshold(2, /treasury/signers))
  }
}
```

---

## Step 5: Commit the Contract

```bash
# Initial commit (no signature needed)
modal c commit --all -m "Treasury contract setup"

# Check status
modal c status
modal c log
```

---

## Step 6: Test the Flow

### Propose a Withdrawal

```bash
# Alice proposes
modal c act PROPOSE --sign alice.passfile
modal c commit --all --sign alice.passfile -m "Alice proposes withdrawal"
```

### First Approval

```bash
# Bob approves
modal c act APPROVE --sign bob.passfile
modal c commit --all --sign bob.passfile -m "Bob approves"
```

### Second Approval & Execute

```bash
# Carol approves (now we have 2-of-3)
# The threshold predicate validates both signatures
modal c act EXECUTE --sign carol.passfile
modal c commit --all --sign carol.passfile -m "Execute withdrawal"
```

---

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

---

## Advanced: Graduated Thresholds

Different actions can have different requirements:

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

---

## Next Steps

- [Oracle Escrow Tutorial](./ORACLE_ESCROW.md) - External verification
- [FOR_AGENTS.md](../FOR_AGENTS.md) - Why verification matters
- [standard-predicates.md](../standard-predicates.md) - All available predicates

---

*Questions? Open a GitHub issue or ask on Discord.* 🔐
