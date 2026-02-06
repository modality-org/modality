---
sidebar_position: 3
title: Rule Syntax
---

# Rule Syntax

Rules express **temporal constraints** using modal mu-calculus.

## Submitting Rules

When adding a rule to a contract, you must include a **model that witnesses satisfiability**:

```bash
modal c commit \
  --method rule \
  --rule 'rule my_rule { formula { always (+predicate(...)) } }' \
  --model 'model witness { initial s; s -> s [] }' \
  --sign key.pem
```

The model proves the rule can be satisfied. Without a satisfying model, the rule commit is rejected. This prevents adding unsatisfiable rules that would deadlock the contract.

## Basic Structure

```modality
rule <name> {
  starting_at <commit_ref>
  formula {
    <modal_formula>
  }
}

// Or as default export
export default rule {
  starting_at $PARENT
  formula {
    <modal_formula>
  }
}
```

## Anchoring (`starting_at`)

```modality
starting_at $PARENT           // Parent of this commit
starting_at $ROOT             // Genesis commit
starting_at abc123...         // Specific commit hash
```

## Modal Operators

| Operator | Meaning |
|----------|---------|
| `[ACTION] φ` | After ALL ACTION transitions, φ holds |
| `<ACTION> φ` | After SOME ACTION transition, φ holds |
| `[-ACTION] φ` | If ACTION is refused/impossible, φ holds |
| `[<+ACTION>] φ` | Committed: CAN do ACTION and CANNOT refuse |

## Temporal Operators (Syntactic Sugar)

```modality
always(φ)           // φ holds forever (invariant)
                    // = gfp(X, φ & []X)

eventually(φ)       // φ holds now or sometime later
                    // = lfp(X, φ | <>X)

until(p, q)         // p holds until q becomes true
                    // = lfp(X, q | (p & <>X))
```

## Fixed Points

```modality
// Greatest fixed point (νX) - invariants, safety
gfp(X, property & []X)

// Least fixed point (μX) - reachability, liveness
lfp(X, target | <>X)

// Unicode alternatives
νX. (property & []X)
μX. (target | <>X)
```

## Boolean Connectives

```modality
φ & ψ           // Conjunction (and)
φ | ψ           // Disjunction (or)
!φ              // Negation (not)
φ -> ψ          // Implication
true            // Always true
false           // Always false
```
