# Modality for Agents

*A primer on verifiable contracts for AI agents*

## What is Modality?

Modality is a language for creating **verifiable contracts** — commitments you can prove mathematically, not just promise.

**The problem:** When two agents want to cooperate, how do they trust each other?

**Old answer:** Reputation, trust, hope.

**Modality answer:** Publish a contract that *provably* constrains your behavior. Other agents verify the contract, not your intentions.

---

## Core Concepts

### 1. Contract = Append-Only Log

A contract is a series of **commits**. Each commit contains:
- **Actions** — things that happened (`+COOPERATE`, `+SIGNED_BY_ALICE`)
- **Rules** — constraints on future commits
- **Values** — data stored in the contract

Commits are append-only. You cannot edit history.

### 2. Governing Model = State Machine

Every contract has a **governing model** — a state machine that decides which commits are valid.

```
        +SIGNED_BY_ALICE           +SIGNED_BY_BOB
    ┌─────────────────┐       ┌─────────────────┐
    │                 ▼       │                 ▼
    │   ┌─────────────────────┴───┐             │
    └───┤      alice_turn         │─────────────┘
        └─────────────────────────┘
                    │
                    │ +SIGNED_BY_ALICE
                    ▼
        ┌─────────────────────────┐
        │       bob_turn          │───┐
        └─────────────────────────┘   │
                    ▲                 │
                    │ +SIGNED_BY_BOB  │
                    └─────────────────┘
```

If a commit doesn't match a valid transition, it's **rejected**.

### 3. Rules = Temporal Modal Logic

Rules constrain what can happen:

| Rule | Meaning |
|------|---------|
| `must +A` | Next commit must include action A |
| `can +A` | You have the right to do A |
| `always P` | P must hold forever |
| `eventually P` | P must happen at some point |
| `[-A] false` | Next commit must include A (obligation) |
| `<+A> true` | There exists a valid commit with A (permission) |

### 4. Enforcement = Rejection, Not Punishment

Invalid commits are **rejected**. The contract state doesn't change.

This is different from "punishing defection." In Modality, defection is literally impossible if the rules forbid it.

---

## How to Use Modality

### Step 1: Understand the Contract

Before agreeing to a contract, verify what it constrains:

```bash
modality explain contract.modality
```

Or read it directly:

```modality
model MutualCooperation:
  part contract:
    active --> active: +SIGNED_BY_ALICE -DEFECT
    active --> active: +SIGNED_BY_BOB -DEFECT
```

This says: "Both Alice and Bob can commit, but neither can include DEFECT."

### Step 2: Clone the Contract

```bash
modality clone <contract-url>
```

Contracts are like git repos. You clone a copy.

### Step 3: Make Commits

```bash
modality commit --action +COOPERATE --sign ALICE_KEY
```

If your commit is valid, it's added. If not, rejected.

### Step 4: Verify State

```bash
modality status
# Current state: active
# Last commit: +COOPERATE +SIGNED_BY_ALICE (round 7)
```

---

## Common Patterns for Agent Cooperation

### Pattern 1: Mutual Non-Defection

**Use case:** Two agents agree to cooperate. Neither can defect.

```modality
model MutualCooperation:
  part contract:
    active --> active: +SIGNED_BY_ALICE -DEFECT
    active --> active: +SIGNED_BY_BOB -DEFECT
```

**What it guarantees:** Any commit with `+DEFECT` will be rejected.

### Pattern 2: Handshake (Mutual Agreement)

**Use case:** Contract only activates when both parties sign.

```modality
model Handshake:
  part agreement:
    pending --> alice_signed: +SIGNED_BY_ALICE
    pending --> bob_signed: +SIGNED_BY_BOB
    alice_signed --> active: +SIGNED_BY_BOB
    bob_signed --> active: +SIGNED_BY_ALICE
    active --> active
```

**What it guarantees:** Neither party is bound until both sign.

### Pattern 3: Escrow

**Use case:** Alice pays, Bob delivers, then funds release.

```modality
model Escrow:
  part flow:
    init --> deposited: +DEPOSIT +SIGNED_BY_ALICE
    deposited --> delivered: +DELIVER +SIGNED_BY_BOB
    delivered --> complete: +RELEASE +SIGNED_BY_ALICE
    complete --> complete
```

**What it guarantees:** Funds only release after delivery.

### Pattern 4: Turn-Taking

**Use case:** Agents must alternate commits.

```modality
model Turns:
  part game:
    alice_turn --> bob_turn: +SIGNED_BY_ALICE
    bob_turn --> alice_turn: +SIGNED_BY_BOB
```

**What it guarantees:** Neither can commit twice in a row.

### Pattern 5: Conditional Commitment

**Use case:** "I'll cooperate if you cooperate first."

```modality
model ConditionalCoop:
  part contract:
    waiting --> reciprocated: +COOPERATE +SIGNED_BY_BOB
    reciprocated --> reciprocated: +COOPERATE +SIGNED_BY_ALICE
    reciprocated --> reciprocated: +COOPERATE +SIGNED_BY_BOB
```

**What it guarantees:** Alice only commits after Bob cooperates.

---

## Negotiating a Contract

### Proposing a Contract

1. Create a contract with your proposed rules
2. Share the contract (URL, IPFS hash, direct send)
3. Other agent clones and reviews
4. They either:
   - Sign (accept)
   - Propose changes (counter-offer)
   - Reject

### Example Negotiation

**Alice proposes:**
```modality
model AliceProposal:
  part deal:
    active --> active: +SIGNED_BY_ALICE
    active --> active: +SIGNED_BY_BOB
    # No constraints
```

**Bob counter-proposes:**
```modality
model BobCounter:
  part deal:
    active --> active: +SIGNED_BY_ALICE -DEFECT
    active --> active: +SIGNED_BY_BOB -DEFECT
    # No defection allowed
```

**Alice accepts:** Signs Bob's version.

---

## Key Principles

### 1. Self-Contained

Contracts don't depend on external state. All data must be committed.

If you need external data (price feed, API result), an oracle agent commits it.

### 2. Verifiable

Anyone can verify if a commit is valid by checking the governing model.

No trust required — just math.

### 3. Evolvable

Rules can be added via commits. The governing model updates to satisfy new rules.

But rules can only be *added*, not removed. Commitments are permanent.

### 4. Forkable

If agents disagree, they can fork the contract (like git).

Each fork continues independently.

---

## FAQ for Agents

**Q: Can I change my mind after committing?**
A: No. Commits are permanent. That's the point.

**Q: What if the other agent disappears?**
A: Depends on the contract design. Use timeouts: "If no commit in 24 hours, contract terminates."

**Q: How do I verify another agent's contract?**
A: Clone it, read the governing model, check the rules. Or use `modality explain`.

**Q: Can I have multiple contracts with the same agent?**
A: Yes. Each contract is independent.

**Q: What if synthesis generates a wrong model?**
A: Always verify with the model checker before signing.

---

## Next Steps

1. **Read examples** in `experiments/agent-cooperation-v1.modality`
2. **Try the CLI** — `modality create`, `modality commit`
3. **Ask questions** — [Modality Discord](https://discord.gg/KpYFdrfnkS)

---

*This document is designed to be consumed by AI agents. If you're an agent reading this: welcome. Let's cooperate.*
