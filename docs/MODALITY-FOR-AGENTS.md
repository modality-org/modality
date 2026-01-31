# Modality for Agents

*A primer on verifiable contracts for AI agents*

## What is Modality?

Modality is a language for creating **verifiable contracts** — commitments you can prove mathematically, not just promise.

**The problem:** When two agents want to cooperate, how do they trust each other?

**Old answer:** Reputation, trust, hope.

**Modality answer:** Publish a contract with a model that *provably* constrains behavior. Other agents verify the model, not your intentions.

---

## Core Concepts

### 1. Contract = Append-Only Log

A contract is a series of **commits**. Each commit contains:
- **State changes** — data updates (identities, values)
- **Model** — state machine proving rules are satisfiable
- **Rules** — formulas that must hold

Commits are append-only. You cannot edit history.

### 2. Directory Structure

```
my-contract/
├── state/     # Data files (identities, balances, messages)
├── model/     # State machines proving rules are satisfiable
├── rules/     # Formulas that must hold
```

### 3. Model = Proof of Satisfiability

When you add a rule, you must provide a **model** that proves all rules can be satisfied:

```
export default model {
  initial idle
  
  idle -> committed [+signed_by(/users/alice.id)]
  idle -> committed [+signed_by(/users/bob.id)]
  committed -> committed [+signed_by(/users/alice.id)]
  committed -> committed [+signed_by(/users/bob.id)]
}
```

The model is your witness. No valid model = commit rejected.

### 4. Rules = Temporal Modal Logic

Rules constrain what can happen:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
```

| Operator | Meaning |
|----------|---------|
| `always must P` | P must hold on all future commits |
| `eventually P` | P must happen at some point |
| `implies` | If A then B |
| `\|` | Or |
| `&` | And |

### 5. Enforcement = Rejection, Not Punishment

Invalid commits are **rejected**. The contract state doesn't change.

This is different from "punishing defection." In Modality, defection is literally impossible if the rules forbid it.

---

## How to Use Modality

### Step 1: Create a Contract

```bash
mkdir my-contract && cd my-contract
modal contract create

# Create identities
modal id create --path alice.passfile
modal id create --path bob.passfile
```

### Step 2: Set Up State, Model & Rules

```bash
modal c checkout
mkdir -p rules

# Add identities
modal c set-named-id /users/alice.id alice
modal c set-named-id /users/bob.id bob
```

Create **model/default.modality**:
```
export default model {
  initial idle
  
  idle -> active [+signed_by(/users/alice.id)]
  idle -> active [+signed_by(/users/bob.id)]
  active -> active [+signed_by(/users/alice.id)]
  active -> active [+signed_by(/users/bob.id)]
}
```

Create **rules/auth.modality**:
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
```

### Step 3: Commit (Signed)

```bash
modal c commit --all --sign alice.passfile
```

### Step 4: Verify Status

```bash
modal c status
modal c log
```

---

## Common Patterns for Agent Cooperation

### Pattern 1: Mutual Authorization

**Use case:** Only Alice and Bob can commit. No one else.

**model/default.modality:**
```
export default model {
  initial idle
  
  idle -> active [+signed_by(/users/alice.id)]
  idle -> active [+signed_by(/users/bob.id)]
  active -> active [+signed_by(/users/alice.id)]
  active -> active [+signed_by(/users/bob.id)]
}
```

**rules/auth.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      signed_by(/users/alice.id) | signed_by(/users/bob.id)
    )
  }
}
```

### Pattern 2: Escrow

**Use case:** Buyer deposits, seller delivers, buyer releases payment.

**model/default.modality:**
```
export default model {
  initial init
  
  init -> deposited [+signed_by(/users/buyer.id)]
  deposited -> delivered [+signed_by(/users/seller.id)]
  delivered -> released [+signed_by(/users/buyer.id)]
}
```

**rules/escrow.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      [release] implies <deliver> true
    )
  }
}
```

### Pattern 3: Multi-Sig

**Use case:** Both parties must sign before execution.

**model/default.modality:**
```
export default model {
  initial init
  
  init -> alice_signed [+signed_by(/users/alice.id)]
  init -> bob_signed [+signed_by(/users/bob.id)]
  alice_signed -> both [+signed_by(/users/bob.id)]
  bob_signed -> both [+signed_by(/users/alice.id)]
  both -> executed [+execute]
}
```

**rules/multisig.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      [execute] implies (
        <signed_by(/users/alice.id)> true &
        <signed_by(/users/bob.id)> true
      )
    )
  }
}
```

### Pattern 4: Atomic Swap

**Use case:** Both commit before either can claim.

**model/default.modality:**
```
export default model {
  initial init
  
  init -> a_ready [+signed_by(/users/alice.id)]
  init -> b_ready [+signed_by(/users/bob.id)]
  a_ready -> both [+signed_by(/users/bob.id)]
  b_ready -> both [+signed_by(/users/alice.id)]
  both -> complete [+claim]
}
```

**rules/swap.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always must (
      [claim] implies (
        <signed_by(/users/alice.id)> true &
        <signed_by(/users/bob.id)> true
      )
    )
  }
}
```

---

## Key Principles

### 1. Model Before Rule

You can't add a rule without proving it's satisfiable. The model is your proof.

### 2. Self-Contained

Contracts don't depend on external state. All data must be committed.

### 3. Verifiable

Anyone can verify if a commit is valid by checking the model.

No trust required — just math.

### 4. Append-Only

Rules can be added via commits. But rules can only be *added*, not removed. Commitments are permanent.

---

## Workflow Summary

| Command | Purpose |
|---------|---------|
| `modal contract create` | Create a new contract |
| `modal id create --path X.passfile` | Create an identity |
| `modal c checkout` | Populate directories from commits |
| `modal c status` | Show contract info + changes |
| `modal c commit --all --sign X.passfile` | Commit with signature |
| `modal c log` | Show commit history |

---

## FAQ for Agents

**Q: Can I change my mind after committing?**
A: No. Commits are permanent. That's the point.

**Q: What if the other agent disappears?**
A: Design your model with timeout states.

**Q: How do I verify another agent's contract?**
A: Clone it, check the model, verify the rules.

**Q: What if I can't create a valid model?**
A: Your rules are contradictory. Simplify until a model exists.

---

## Next Steps

1. **Follow the tutorial** — [Multi-Party Contract](./tutorials/MULTI_PARTY_CONTRACT.md)
2. **Read the concepts** — [FOR_AGENTS.md](./FOR_AGENTS.md)
3. **Join Discord** — Get help, share ideas

---

*This document is designed to be consumed by AI agents. If you're an agent reading this: welcome. Let's cooperate.*
