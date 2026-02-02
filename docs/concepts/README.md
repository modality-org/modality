# Core Concepts

This guide explains the fundamental concepts behind Modality.

## Table of Contents

1. [Contracts as Append-Only Logs](#contracts-as-append-only-logs)
2. [State Machines (Models)](#state-machines-models)
3. [Temporal Modal Logic (Rules)](#temporal-modal-logic-rules)
4. [Cryptographic Predicates](#cryptographic-predicates)
5. [Potentialist State Machines](#potentialist-state-machines)

---

## Contracts as Append-Only Logs

A Modality contract is an **append-only log of signed commits**. Each commit can:
- Add or modify **state** (data files)
- Add **rules** (temporal formulas that constrain behavior)
- Perform **actions** (state transitions in the model)

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

### Key Properties

- **Immutable**: Once committed, history cannot change
- **Ordered**: Commits form a linear sequence
- **Signed**: Each commit is cryptographically signed
- **Validated**: Action commits are validated against ALL accumulated rules

---

## State Machines (Models)

A **model** defines the allowed behaviors as a labeled transition system (LTS):

```modality
model escrow {
  states { pending, funded, delivered, released, refunded }
  initial pending
  terminal released, refunded
  
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/buyer.id)
  
  transition DELIVER: funded -> delivered
    +signed_by(/parties/seller.id)
  
  transition RELEASE: delivered -> released
    +signed_by(/parties/buyer.id)
}
```

### Components

| Component | Description |
|-----------|-------------|
| `states` | All possible states the contract can be in |
| `initial` | The starting state |
| `terminal` | End states (self-loop implied) |
| `transition` | A labeled edge: `ACTION: from -> to` |
| `+predicate` | Conditions that must hold for the transition |

### Why State Machines?

State machines make verification tractable:
- **Finite** — We can enumerate all states
- **Deterministic** — Given state + action → next state
- **Composable** — Multiple parties add their constraints
- **Verifiable** — Model checkers can prove properties

---

## Temporal Modal Logic (Rules)

Rules express constraints using **modal mu-calculus** — a logic that reasons about what's possible, necessary, and how things evolve over time.

```modality
rule buyer_protection {
  starting_at $PARENT
  formula {
    always (
      [<+RELEASE>] eventually delivered
    )
  }
}
```

### Modal Operators

| Operator | Meaning |
|----------|---------|
| `[A] φ` | After ALL A-transitions, φ holds |
| `<A> φ` | After SOME A-transition, φ holds |
| `[-A] φ` | If A is refused, φ holds |
| `[<+A>] φ` | Committed: must do A, and φ holds after |

### Temporal Operators (Sugar)

| Operator | Meaning | Definition |
|----------|---------|------------|
| `always φ` | φ holds now and forever | `gfp(X, φ & []X)` |
| `eventually φ` | φ holds now or sometime later | `lfp(X, φ \| <>X)` |
| `until(p, q)` | p holds until q becomes true | `lfp(X, q \| (p & <>X))` |

### Fixed Points

For complex properties, use explicit fixed points:

```modality
// Greatest fixed point: invariant
gfp(X, some_property & []X)

// Least fixed point: reachability  
lfp(X, target | <>X)
```

---

## Cryptographic Predicates

Predicates bind **real-world identity** to **logical constraints**:

### Signature Verification

```modality
+signed_by(/parties/alice.id)
```

Requires the commit to be signed by the ed25519 key at that path.

### Threshold Signatures (Multisig)

```modality
+threshold(/signers, 2, 3)
```

Requires 2-of-3 signers from the list.

### Oracle Attestations

```modality
+oracle_attests(/oracles/price-feed.id, "price > 100")
```

Requires an external oracle to attest to a condition.

### Timestamps

```modality
+after(/deadlines/expiry.datetime)
+before(/deadlines/cutoff.datetime)
```

Time-based constraints.

### Hash Commitments

```modality
+hash_matches(/commitments/secret.hash, revealed_value)
```

For commit-reveal schemes.

---

## Potentialist State Machines

The deepest concept: a contract is not a fixed state machine — it's one **actualization** from a space of **potential** state machines.

### The Insight

When Alice and Bob start a contract:
1. **Potential is infinite** — Any state machine is possible
2. **Alice adds a rule** — Potential shrinks to state machines satisfying her rule
3. **Bob adds a rule** — Potential shrinks further
4. **Rules accumulate** — The space of valid futures only contracts, never expands

### Monotonicity Theorem

> Adding a covenant (rule) can only shrink the space of valid extensions, never expand it.

This is why Modality contracts are safe:
- Each party adds their protection
- No one can add a rule that invalidates existing protections
- The final contract satisfies ALL parties' constraints

### Model Witnesses

When you add a rule, you must provide a **model witness** — an explicit state machine that demonstrates the rule is satisfiable. This prevents "impossible rules" that would deadlock the contract.

---

## Summary

| Concept | Purpose |
|---------|---------|
| **Append-only log** | Immutable, ordered history |
| **State machines** | Finite, verifiable behaviors |
| **Modal logic** | Express temporal constraints |
| **Predicates** | Bind identity to logic |
| **Potentialism** | Safe accumulation of rules |

## Next Steps

- **[Language Reference](../language/README.md)** — Syntax details
- **[CLI Reference](../cli/README.md)** — Working with contracts
- **[Tutorials](../tutorials/)** — Hands-on examples
