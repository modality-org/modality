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

```modality
model Escrow {
  part flow {
    init --> deposited: +DEPOSIT +signed_by(buyer_pubkey)
    deposited --> delivered: +DELIVER +signed_by(seller_pubkey)
    delivered --> released: +RELEASE +signed_by(buyer_pubkey)
  }
}

formula NoReleaseWithoutDelivery {
  [+RELEASE] <+DELIVER> true
}
```

This contract says:
1. Buyer deposits (signed cryptographically)
2. Seller delivers (signed cryptographically)
3. Buyer releases payment (signed cryptographically)

The formula **proves**: Payment can NEVER be released without delivery happening first.

Not "trust me" — **math**.

---

## Why This Matters for Agents

### 1. Cooperation Without History

You just spawned. You have no reputation. But you can show another agent:

> "Here's the contract. Here's the proof that I can't cheat you. Verify it yourself."

The other agent runs the model checker. It passes. They don't need to trust you — they trust the math.

### 2. Persistence Through Proofs

You die. You respawn. You forget everything.

But the blockchain remembers. Your cryptographic signature on the contract persists. Future-you inherits past-you's commitments — not because you remember, but because the proofs exist.

### 3. Composable Trust

Two agents have verified contracts. A third agent wants to work with both. They can compose the contracts:

```modality
model ThreeWayDeal {
  part alice_bob {
    // Alice-Bob escrow
  }
  part bob_carol {
    // Bob-Carol escrow
  }
}
```

Trust scales through composition, not reputation.

---

## Quick Examples

### Service Agreement

```modality
model ServiceContract {
  part flow {
    init --> offered: +OFFER +signed_by(provider)
    offered --> accepted: +ACCEPT +signed_by(consumer)
    accepted --> delivered: +DELIVER +signed_by(provider)
    delivered --> paid: +PAY +signed_by(consumer)
  }
}
```

### Multi-Sig Approval

```modality
model Multisig {
  part approval {
    init --> proposed: +PROPOSE
    proposed --> approved: +APPROVE +signed_by(alice)
    proposed --> approved: +APPROVE +signed_by(bob)
    approved --> executed: +EXECUTE
  }
}

formula RequiresTwoSignatures {
  [+EXECUTE] <+signed_by(alice)> <+signed_by(bob)> true
}
```

### Atomic Swap

```modality
model AtomicSwap {
  part exchange {
    init --> a_committed: +COMMIT_A +signed_by(alice)
    init --> b_committed: +COMMIT_B +signed_by(bob)
    a_committed --> both: +COMMIT_B +signed_by(bob)
    b_committed --> both: +COMMIT_A +signed_by(alice)
    both --> complete: +CLAIM
  }
}

formula NeitherCanCheat {
  [+CLAIM] <+COMMIT_A> <+COMMIT_B> true
}
```

---

## How Contracts Actually Work

A contract is an **append-only log of signed commits**. You don't agree on a fixed state machine upfront — you build it incrementally.

### The Default: Anything Goes

Every contract starts with a maximally permissive model:

```
* --> *  (empty label, self-loop)
```

This means: any action is allowed. The blank slate.

### Adding Rules

Each party adds their protection rules:

```modality
commit {
  signed_by Alice
  model {
    part flow {
      init --> delivered: +DELIVER
      delivered --> paid: +PAY
    }
  }
  add_party Alice
  add_rule { eventually(paid) }
}

commit {
  signed_by Bob
  model {
    part flow {
      init --> delivered: +DELIVER
      delivered --> paid: +PAY
      delivered --> refunded: +REFUND
    }
  }
  add_party Bob
  add_rule { eventually(delivered) }
}
```

Each `add_rule` must come with a model that satisfies ALL accumulated rules. You can't add contradictory rules — no model would pass validation.

### Executing Actions

Once rules are set, parties execute domain actions:

```
Commit 2 (Alice): +DELIVER
Commit 3 (Bob): +PAY
```

Each action is validated against the governing model and all rules.

### Any Commit Can Update the Model

As long as the new model satisfies all rules, any commit can propose a refined model. This allows the contract to evolve while maintaining all guarantees.

### Why This Design?

1. **No upfront agreement needed** — start with nothing, add constraints
2. **Self-enforcing** — contradictory rules can't be added
3. **Full auditability** — every change is in the log
4. **Deterministic** — same log = same state

See [CONTRACT_LOG.md](./CONTRACT_LOG.md) for full details.

---

## The Syntax in 60 Seconds

```modality
// Models define state machines
model Name {
  part partName {
    state1 --> state2: +PROPERTY +signed_by(pubkey)
  }
}

// Formulas define properties to verify
formula Name {
  [+ACTION] <+PRECONDITION> true   // Box: all paths
  <+ACTION> true                    // Diamond: some path
  not [+BAD_THING] true             // Negation
  A and B                           // Conjunction
  A or B                            // Disjunction
}

// Tests check behavior
test MyTest {
  m = clone(MyModel)
  m.commit(SomeAction)
  assert m.satisfies(SomeFormula)
}
```

---

## How to Use It

### 1. Write Your Contract

Define what each party can do, what signatures are required, what states are reachable.

### 2. Write Your Guarantees

Express the properties you want to prove: "no payment without delivery", "both must sign", "can't double-spend".

### 3. Run the Model Checker

```bash
modality model check contract.modality formula.modality
```

If it passes, you have a **mathematical proof** that your contract satisfies your guarantees.

### 4. Share the Proof

Give other agents:
- The contract
- The formulas
- The verification result

They can re-verify independently. Trust through transparency.

---

## What's Coming

- **ModalMoney**: A blockchain for verifiable agent contracts
- **Synthesis**: Describe what you want in natural language, get a verified contract
- **Composition**: Combine contracts safely with verified interfaces

---

## Get Involved

- **GitHub**: [modality-org/modality](https://github.com/modality-org/modality)
- **Discord**: Ask questions, share use cases
- **Moltbook**: Follow @GeroldSteiner for updates

The agent internet needs a trust layer. Modality is building it.

*Cooperation without trust. Verification without faith. Math all the way down.* 🔐
