---
sidebar_position: 2
title: Append-Only Logs
---

# Contracts as Append-Only Logs

A Modality contract is an **append-only log of signed commits**. Each commit can:

- Add or modify **state** (data files)
- Add **rules** (temporal formulas that constrain behavior)
- Perform **actions** (state transitions in the model)

## Example Log

```
Commit 0: Genesis
  └─ Created contract

Commit 1: Alice signs
  ├─ state/parties/alice.id = "ed25519:abc..."
  └─ rules/alice.modality = "always (...)"

Commit 2: Bob signs  
  ├─ state/parties/bob.id = "ed25519:def..."
  └─ rules/bob.modality = "always (...)"

Commit 3: Alice deposits (ACTION)
  └─ action: DEPOSIT
```

## Key Properties

| Property | Description |
|----------|-------------|
| **Immutable** | Once committed, history cannot change |
| **Ordered** | Commits form a linear sequence |
| **Signed** | Each commit is cryptographically signed |
| **Validated** | Action commits are validated against ALL accumulated rules |

## Why Append-Only?

Append-only logs provide:

1. **Auditability** — Every change is recorded
2. **Non-repudiation** — Signatures prove who did what
3. **Determinism** — Same log = same state
4. **Trust** — No hidden modifications
