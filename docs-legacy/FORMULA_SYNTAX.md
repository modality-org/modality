# Modality Formula Syntax

Complete reference for temporal modal logic formulas in Modality.

---

## Overview

Modality formulas are based on **modal mu-calculus** — a powerful logic for expressing properties over state machines. The syntax supports:

- Boolean operators (and, or, not, implies)
- Modal operators (box, diamond, diamondbox)
- Temporal operators (always, eventually, until, next)
- Fixed point operators (lfp, gfp)

---

## Boolean Operators

| Syntax | Meaning |
|--------|---------|
| `true` | Always true |
| `false` | Always false |
| `P` | Proposition P holds |
| `P & Q` or `P and Q` | Both P and Q hold |
| `P \| Q` or `P or Q` | Either P or Q holds |
| `!P` or `not P` | P does not hold |
| `P -> Q` or `P implies Q` | If P then Q |
| `(P)` | Parentheses for grouping |

---

## Modal Operators

Modal operators reason about **transitions** in the state machine.

### Box (Necessity): `[action] φ`

"For ALL transitions labeled with `action`, the target state satisfies `φ`"

```modality
[+PAY] delivered    // All PAY transitions lead to delivered state
[-CANCEL] true      // No CANCEL transition exists (vacuously true if no -CANCEL)
```

### Diamond (Possibility): `<action> φ`

"There EXISTS a transition labeled with `action` to a state satisfying `φ`"

```modality
<+PAY> delivered    // There's a PAY transition to delivered
<+signed_by(X)> true // X can sign (transition exists)
```

### Unlabeled Box/Diamond: `[] φ` and `<> φ`

Reason about ALL transitions regardless of label:

```modality
[] safe           // All successor states are safe
<> goal           // Some successor state is goal
```

### Diamondbox (Commitment): `[<action>] φ`

"Can do `action` AND cannot refuse" — the commitment operator.

Semantically equivalent to: `[-action] false & <+action> φ`

```modality
[<+PAY>] true     // Committed to PAY: can pay AND cannot refuse
[<+signed_by(alice)>] true   // Alice is committed to signing
```

**Use case:** Express irrevocable commitments:

```modality
always (
  [<+signed_by(/users/alice.id)>] true | [<+signed_by(/users/bob.id)>] true
)
// At every state, either Alice or Bob is committed to signing
```

---

## Temporal Operators

Temporal operators reason about **paths** through the state machine.

### Always: `always(φ)` or `□(φ)`

"On ALL paths, `φ` holds at EVERY state"

```modality
always(safe)           // Safety invariant: always safe
always([execute] implies signed)  // Execute always requires signature
```

**Semantics:** `always(f) ≡ gfp(X, []X & f)` (greatest fixed point)

### Eventually: `eventually(φ)` or `◇(φ)`

"On SOME path, `φ` holds at SOME future state"

```modality
eventually(complete)   // Can eventually reach completion
eventually(<+PAY> true) // Payment becomes possible eventually
```

**Semantics:** `eventually(f) ≡ lfp(X, <>X | f)` (least fixed point)

### Until: `P until Q`

"P holds until Q becomes true (and Q must eventually become true)"

```modality
pending until complete   // Pending until completion
safe until goal          // Safe until we reach goal
```

**Semantics:** `until(p, q) ≡ lfp(X, q | (p & <>X))`

### Next: `next(φ)`

"In the NEXT state, `φ` holds"

```modality
next(ready)            // Next state is ready
next([+PAY] complete)  // From next state, PAY leads to complete
```

---

## Fixed Point Operators (Modal Mu-Calculus)

For advanced users who want direct access to the underlying semantics.

### Least Fixed Point: `lfp(X, φ)` or `μX.φ`

The smallest set of states satisfying `X = φ[X]`.

Used for **reachability** properties (something eventually happens).

```modality
lfp(X, goal | <>X)     // Same as eventually(goal)
lfp(X, q | (p & <>X))  // Same as p until q
```

### Greatest Fixed Point: `gfp(X, φ)` or `νX.φ`

The largest set of states satisfying `X = φ[X]`.

Used for **invariant** properties (something always holds).

```modality
gfp(X, safe & []X)     // Same as always(safe)
gfp(X, live & []X)     // Liveness invariant
```

### Variable Reference: `X`

Inside a fixed point, refer to the bound variable:

```modality
gfp(X, safe & []X)     // X refers to "states where always(safe)"
                       // Read as: "safe here, and all successors satisfy X"
```

---

## Predicates

Predicates are checked dynamically via WASM modules.

### Signature Verification

```modality
+signed_by(/users/alice.id)   // Requires Alice's signature
-signed_by(/users/bob.id)     // Bob has NOT signed
```

In formulas:
```modality
<+signed_by(/users/alice.id)> true  // Alice CAN sign
[+signed_by(/users/bob.id)] complete  // Bob signing leads to complete
```

---

## Rule File Structure

Rules are defined in `.modality` files:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [+execute] implies (
        <+signed_by(/users/alice.id)> true &
        <+signed_by(/users/bob.id)> true
      )
    )
  }
}
```

- `starting_at $PARENT` — rule applies from this commit forward
- `formula { ... }` — the temporal logic formula

---

## Examples

### Authorization Rule

"Every action must be signed by Alice or Bob"

```modality
formula {
  always (
    [<+signed_by(/users/alice.id)>] true | [<+signed_by(/users/bob.id)>] true
  )
}
```

### Escrow Safety

"Release can only happen after delivery"

```modality
formula {
  always (
    [+RELEASE] implies <+DELIVER> true
  )
}
```

### Multi-sig Requirement

"Execute requires both signatures"

```modality
formula {
  always (
    [+EXECUTE] implies (
      <+signed_by(/users/alice.id)> true &
      <+signed_by(/users/bob.id)> true
    )
  )
}
```

### Eventual Completion

"From any state, completion is reachable"

```modality
formula {
  always(eventually(complete))
}
```

### Custom Fixed Point

"All states can reach a checkpoint infinitely often" (using raw mu-calculus)

```modality
formula {
  gfp(Y, lfp(X, checkpoint | <>X) & []Y)
}
```

---

## Operator Precedence

From highest to lowest:
1. `()` — parentheses
2. `!`, `not` — negation
3. `[]`, `<>`, `[<>]` — modal operators
4. `always`, `eventually`, `next`, `lfp`, `gfp` — temporal/fixed point
5. `&`, `and` — conjunction
6. `|`, `or` — disjunction
7. `->`, `implies` — implication
8. `until` — temporal until

---

## Summary Table

| Operator | Syntax | Meaning |
|----------|--------|---------|
| Box | `[a] φ` | All a-transitions satisfy φ |
| Diamond | `<a> φ` | Some a-transition satisfies φ |
| Diamondbox | `[<a>] φ` | Committed: can do a, cannot refuse |
| Always | `always(φ)` | φ holds on all paths forever |
| Eventually | `eventually(φ)` | φ holds on some path eventually |
| Until | `p until q` | p holds until q (q must occur) |
| Next | `next(φ)` | φ holds in next state |
| LFP | `lfp(X, φ)` | Least fixed point of X = φ |
| GFP | `gfp(X, φ)` | Greatest fixed point of X = φ |

---

*Modal mu-calculus: where math meets trust.* 🔐
