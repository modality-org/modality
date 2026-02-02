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

## 4. Define the Model

Create `model/escrow.modality`:

```modality
model escrow {
  states { pending, funded, delivered, released, refunded }
  initial pending
  terminal released, refunded
  
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/alice.id)
  
  transition DELIVER: funded -> delivered
    +signed_by(/parties/bob.id)
  
  transition RELEASE: delivered -> released
    +signed_by(/parties/alice.id)
  
  transition REFUND: funded -> refunded
    +signed_by(/parties/bob.id)
}
```

The model structure itself enforces ordering: RELEASE can only happen from `delivered` state, so DELIVER must precede RELEASE.

## 5. Commit and Verify

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
