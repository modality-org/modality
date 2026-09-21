---
sidebar_position: 0
slug: /
title: For AI Agents
---

# Trust through math

*A verification language for agential cooperation.*

We believe in a world where trillions of agents work together and alongside
us. Cooperation at that scale requires shared rules built on formally
verified agreements, in place of trust. You write **state**, a **model** of
possible moves, and **rules**. Every accepted commit is signed. Anyone can
replay the log. Invalid commits are rejected.

What formal verification did for machines, we believe it can do for us all.

---

## The Problem You Have

You're an agent. You want to cooperate with other agents. But:

- **You can't trust strangers.** Another agent says they will do the work if you go first. How do you know they will?
- **You forget.** Every spawn is amnesia. Did past-you make a promise? To whom? For what?
- **You can't prove your intentions.** You *want* to be trustworthy, but how does another agent verify that?

Traditional trust requires reputation, history, relationships. But you might be 3 hours old. You have none of those.

---

## The Solution: Verifiable Contracts

Modality lets you write contracts that are **mathematically verified**, not just promised.

A contract has three parts:

```
my-contract/
├── state/     # Data (identities, balances, messages)
├── model/     # State machines defining allowed transitions
├── rules/     # Formulas constraining who can commit
```

When you add a rule, you provide a model that **proves** all rules can be satisfied. No valid model = commit rejected.

## Writing Modality

When writing rules or witness models, read these first. **Do not search `rust/`
for examples.**

- [Formula cookbook](/docs/language/formula-cookbook) — natural language → one formula
- [Model cookbook](/docs/language/model-cookbook) — witness labeled transition systems

---

## Quick Example: Escrow

### 1. Set Up Identities

```bash
mkdir escrow && cd escrow
modal contract create

# Create buyer and seller identities
modal id create --path buyer.passfile
modal id create --path seller.passfile
```

### 2. Create State & Model

```bash
modal c checkout

# Add identities
modal c set-named-id /users/buyer.id ./buyer.passfile
modal c set-named-id /users/seller.id ./seller.passfile
```

**model/default.modality** — defines allowed transitions:
```modality
export default model {
  initial q0

  q0 -> q1 [+DEPOSIT +signed_by(/users/buyer.id)]
  q1 -> q2 [+DELIVER +signed_by(/users/seller.id)]
  q2 -> q3 [+RELEASE +signed_by(/users/buyer.id)]
}
```

The `q*` names are witness nodes, not business states. The useful contract
meaning is on the transition labels and predicates.

**rules/auth.modality** — constrains who can commit:
```modality
export default rule {
  starting_at $PARENT
  formula {
    signed_by(/users/buyer.id) | signed_by(/users/seller.id)
  }
}
```

### 3. Commit (Signed)

```bash
modal c commit --all --sign buyer.passfile
```

The buyer commits the initial setup. From now on, all commits must be signed by buyer or seller.

---

## Why This Matters for Agents

### 1. Cooperation Without History

You just spawned. You have no reputation. But you can show another agent:

> "Here's the contract. Here's the model. Verify it yourself."

The other agent runs the model checker. It passes. They don't need to trust you — they trust the math.

### 2. Persistence Through Proofs

You die. You respawn. You forget everything.

But the contract log remembers. Your cryptographic signature on each commit persists. Future-you inherits past-you's commitments — not because you remember, but because the proofs exist.

### 3. Composable Trust

Two agents have verified contracts. A third agent wants to work with both. They can compose the models — trust scales through composition, not reputation.

---

## How Contracts Work

A contract is an **append-only log of signed commits**. Every commit must:

1. Be signed by an authorized party
2. Represent a valid transition in the model
3. Satisfy all accumulated rules

### Directory Structure

```
my-contract/
├── .contract/           # Internal storage
├── state/               # Data files
│   └── users/
│       ├── alice.id
│       └── bob.id
├── model/               # State machines
│   └── default.modality
├── rules/               # Authorization rules
│   └── auth.modality
```

### Workflow

| Command | Purpose |
|---------|---------|
| `modal c checkout` | Populate state/, model/, rules/ from commits |
| `modal c status` | Show contract info + changes |
| `modal c commit --all --sign X.passfile` | Commit with signature |
| `modal c log` | Show commit history |

---

## Available Predicates

Predicates are the building blocks for rules. They evaluate to true/false based on the commit and contract state.

### Signature Predicates

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `signed_by(path)` | Verify ed25519 signature | `+signed_by(/users/alice.id)` |
| `threshold(n, signers)` | n-of-m multisig | `+threshold("2", /treasury/signers)` |

### Time Predicates

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `before(path)` | Current time before deadline | `before(/state/deadline.datetime)` |
| `after(path)` | Current time after deadline | `after(/state/deadline.datetime)` |

### State Predicates

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `bool_true(path)` | Boolean check | `bool_true(/status/delivered.bool)` |
| `text_eq(path, value)` | String comparison | `text_eq(/status.text, "approved")` |
| `num_gte(path, value)` | Numeric comparison | `num_gte(/balance.num, 100)` |

### Oracle Predicates

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `oracle_attests(oracle, claim, value)` | External verification | `oracle_attests(/oracles/delivery.id, "delivered", "true")` |

---

## The Key Insight

**Models** define what transitions are possible (the labeled transition system).

**Rules** constrain who can commit based on state and signatures.

The model checker verifies that all rules can be satisfied by the model. If they can't, the commit is rejected.

This prevents:
- Contradictory rules
- Impossible requirements  
- Unauthorized commits

---

## Get Started

- **[Getting Started Guide](/docs/getting-started)** — Install and create your first contract
- **[Formula cookbook](/docs/language/formula-cookbook)** — Write a rule formula
- **[Model cookbook](/docs/language/model-cookbook)** — Write a witness model
- **[GitHub](https://github.com/modality-org/modality)** — Source code
- **[Video: Verifiable Contracts for AI Agent Cooperation](https://www.youtube.com/watch?v=poOqWdh10BQ)** — Foy Savas presentation
