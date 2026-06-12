---
sidebar_position: 3
title: Your First Contract
---

# Your First Contract in 5 Minutes

## 1. Create a Contract

```bash
mkdir my-escrow && cd my-escrow
modal contract create
```

This creates a `.contract/` directory to track commits.

## 2. Create Identities

```bash
modal id create --path alice.passfile
modal id create --path bob.passfile
```

## 3. Initialize State

```bash
modal c checkout
mkdir -p state rules

# Add party identities
modal c set /parties/alice.id $(modal id get --path ./alice.passfile)
modal c set /parties/bob.id $(modal id get --path ./bob.passfile)
```

## 4. Define the Witness Model

Create `model/escrow.modality`:

```modality
model escrow {
  initial q0

  q0 -> q1 [+DEPOSIT +signed_by(/parties/alice.id)]
  q1 -> q2 [+DELIVER +signed_by(/parties/bob.id)]
  q2 -> q3 [+RELEASE +signed_by(/parties/alice.id)]
  q1 -> q4 [+REFUND +signed_by(/parties/bob.id)]
}
```

`q0`, `q1`, and the other node ids are opaque witness nodes. The contract
meaning is carried by labels such as `+DEPOSIT` and predicates such as
`+signed_by(...)`.

## 5. Add Protection Rules

Rules constrain which labeled transitions and commits remain acceptable. Create `rules/buyer-protection.modality`:

```modality
export default rule {
  starting_at $PARENT
  formula {
    signed_by(/parties/alice.id) | signed_by(/parties/bob.id)
  }
}
```

This says: "Every commit must be signed by Alice or Bob."

For authorization based on recorded contract data:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // Only buyer can commit while the delivered flag is false.
    !bool_true(/status/delivered.bool) -> signed_by(/parties/buyer.id)
  }
}
```

## 6. Commit and Verify

```bash
# Commit all changes
modal c commit --all --sign alice.passfile -m "Initial escrow setup"

# Check status
modal c status
```

## What's Next?

- [Core Concepts](/docs/concepts) — Understand the theory
- [CLI Reference](/docs/cli) — All commands explained
- [Language Reference](/docs/language) — Model and rule syntax
