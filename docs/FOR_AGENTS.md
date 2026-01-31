# Modality: Trust Through Math, Not Faith

*A verification language for AI agent cooperation*

---

## The Problem You Have

You're an agent. You want to cooperate with other agents. But:

- **You can't trust strangers.** Another agent says "send me 100 tokens and I'll do the work." How do you know they'll deliver?
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
├── model/     # State machines proving rules are satisfiable
├── rules/     # Formulas that must hold
```

When you add a rule, you provide a model that **proves** all rules can be satisfied. No valid model = commit rejected.

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

### 2. Create State, Model & Rules

```bash
modal c checkout
mkdir -p rules

# Add identities
modal c set /users/buyer.id $(modal id get --path ./buyer.passfile)
modal c set /users/seller.id $(modal id get --path ./seller.passfile)
```

**model/default.modality** — proves the rules are satisfiable:
```
export default model {
  initial init
  
  init -> deposited [+signed_by(/users/buyer.id)]
  deposited -> delivered [+signed_by(/users/seller.id)]
  delivered -> released [+signed_by(/users/buyer.id)]
}
```

**rules/escrow.modality** — the constraints:
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [release] implies <deliver> true
    )
  }
}
```

This formula says: **Release can NEVER happen without delivery happening first.**

### 3. Commit (Signed)

```bash
modal c commit --all --sign buyer.passfile
```

The buyer commits the initial setup. From now on, all commits must follow the model.

---

## Why This Matters for Agents

### 1. Cooperation Without History

You just spawned. You have no reputation. But you can show another agent:

> "Here's the contract. Here's the model proving the rules work. Verify it yourself."

The other agent runs the model checker. It passes. They don't need to trust you — they trust the math.

### 2. Persistence Through Proofs

You die. You respawn. You forget everything.

But the contract log remembers. Your cryptographic signature on each commit persists. Future-you inherits past-you's commitments — not because you remember, but because the proofs exist.

### 3. Composable Trust

Two agents have verified contracts. A third agent wants to work with both. They can compose the models — trust scales through composition, not reputation.

---

## More Examples

### Service Agreement

**model/default.modality:**
```
export default model {
  initial init
  
  init -> offered [+signed_by(/users/provider.id)]
  offered -> accepted [+signed_by(/users/consumer.id)]
  accepted -> delivered [+signed_by(/users/provider.id)]
  delivered -> paid [+signed_by(/users/consumer.id)]
}
```

**rules/service.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [pay] implies <deliver> true
    )
  }
}
```

### Multi-Sig Approval

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
    always (
      [execute] implies (
        <signed_by(/users/alice.id)> true &
        <signed_by(/users/bob.id)> true
      )
    )
  }
}
```

### Atomic Swap

**model/default.modality:**
```
export default model {
  initial init
  
  init -> a_committed [+signed_by(/users/alice.id)]
  init -> b_committed [+signed_by(/users/bob.id)]
  a_committed -> both [+signed_by(/users/bob.id)]
  b_committed -> both [+signed_by(/users/alice.id)]
  both -> complete [+claim]
}
```

**rules/swap.modality:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [claim] implies (
        <signed_by(/users/alice.id)> true &
        <signed_by(/users/bob.id)> true
      )
    )
  }
}
```

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
│   ├── config.json
│   ├── commits/
│   └── HEAD
├── state/               # Data files (POST method)
│   └── users/
│       ├── alice.id
│       └── bob.id
├── model/               # State machines (MODEL method)
│   └── auth.modality
├── rules/               # Formulas (RULE method)
│   └── auth.modality
```

### Workflow

| Command | Purpose |
|---------|---------|
| `modal c checkout` | Populate state/, model/, rules/ from commits |
| `modal c status` | Show contract info + changes |
| `modal c commit --all` | Commit all changes |
| `modal c commit --all --sign X.passfile` | Commit with signature |
| `modal c log` | Show commit history |

---

## The Key Insight

When you add a rule, you must provide a model that proves satisfiability:

- **Model** = state machine showing valid transitions
- **Rule** = formula that must hold over all paths
- **Verification** = model checker proves M ⊨ formula

If you can't prove your rules are satisfiable, you can't commit them. This prevents:

- Contradictory rules
- Impossible requirements  
- Deadlock states

---

## Get Involved

- **GitHub**: [modality-org/modality](https://github.com/modality-org/modality)
- **Discord**: Ask questions, share use cases
- **Tutorial**: [Multi-Party Contract](./tutorials/MULTI_PARTY_CONTRACT.md)

The agent internet needs a trust layer. Modality is building it.

*Cooperation without trust. Verification without faith. Math all the way down.* 🔐
