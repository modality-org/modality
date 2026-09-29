---
sidebar_position: 2
title: Multisig Treasury
---

# Building a Multisig Treasury Contract

Learn to create a 2-of-3 multisig treasury using the `threshold` predicate.

## What We're Building

A treasury contract where:
- 3 keyholders control the funds
- Every commit after the first needs a keyholder's signature
- Any keyholder can post a withdrawal proposal
- A withdrawal, or a change to a keyholder's key, needs 2 of the 3 keyholders
  on the same commit

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
modal c set-named-id /treasury/alice.id alice
modal c set-named-id /treasury/bob.id bob
modal c set-named-id /treasury/carol.id carol
```

`threshold("2", /treasury)` counts the keys in the `*.id` files under
`/treasury`, so the keyholder list is those three files. Proposals live
outside `/treasury`, under `/proposals`; executed withdrawals live under
`/treasury/withdrawals`.

## Step 4: Define the Rules

Create `rules/treasury-auth.modality`. Every commit after the one that adds it
must carry a keyholder's signature:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([-any_signed(/treasury)] false)
  }
}
```

Create `rules/treasury-threshold.modality`. A commit that writes anything under
`/treasury`, a withdrawal or a key, must carry two keyholders' signatures:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([+modifies(/treasury) -threshold("2", /treasury)] false)
  }
}
```

The second rule covers the keys too. Without it, Alice could sign a commit that
replaces Bob's key with a second key of her own, and then meet the threshold
alone.

## Step 5: Write the Witness Model

The model shows the rules can be met. The first commit installs the keys, the
rules and the model, so its edge is unlabeled. After that, a step either leaves
`/treasury` alone and has one keyholder's signature, or has two.

`model/treasury.modality`:

```modality
model Treasury {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/treasury) -modifies(/treasury)
    q1 --> q1: +any_signed(/treasury) +threshold("2", /treasury)
  }
}
```

The rules, not the model, protect the treasury. A later `MODEL` commit is
judged by the model it posts, so a replacement that drops the threshold edge
fails the second rule and is refused.

You can ask the synthesizer for a candidate model and check it against a rule:

```bash
modality model synthesize --rule rules/treasury-threshold.modality --verify -o model/candidate.modality
```

Review any candidate against both rules before you commit it.

## Step 6: Commit and Test

```bash
modal c commit --all --sign alice -m "Initialize treasury"
```

### Propose a Withdrawal

Any keyholder can propose:

```bash
mkdir -p state/proposals
echo '{"amount": 100, "to": "recipient_address"}' > state/proposals/withdrawal.json
modal c commit --all --sign alice -m "Alice proposes withdrawal"
```

### Execute with Two Keyholders

The withdrawal commit carries both signatures:

```bash
mkdir -p state/treasury/withdrawals
cp state/proposals/withdrawal.json state/treasury/withdrawals/0001.json
modal c commit --all --sign bob --sign carol -m "Execute withdrawal"
```

The same commit with only `--sign bob` is refused. Approvals in separate
commits do not add up: `threshold` counts the signatures on one commit.

## How Threshold Works

The `threshold("n", /path)` predicate:

1. Reads the keys in the accepted `*.id` files at `/path` and below
2. Collects the signatures on the pending commit
3. Counts the unique signers whose keys are on that list
4. Holds when the count is at least `n`

**Key features:**
- The same signer counts once
- Signatures from keys not on the list do not count
- Works with any n-of-m configuration

## Available Templates

List all synthesis templates:

```bash
modality model synthesize --list
```

Templates include: `escrow`, `handshake`, `mutual_cooperation`, `atomic_swap`, `multisig`, `service_agreement`, `delegation`, `auction`, `subscription`, `milestone`.
