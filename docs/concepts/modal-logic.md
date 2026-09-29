---
sidebar_position: 4
title: Modal Logic
---

# Temporal Modal Logic (Rules)

Rules express constraints using **modal mu-calculus** — a logic that reasons about what's possible, necessary, and how things evolve over time.

## Modal Operators

| Operator | Meaning |
|----------|---------|
| `[A] φ` | After ALL A-transitions, φ holds |
| `<A> φ` | After SOME A-transition, φ holds |
| `[-A] φ` | If A is refused/impossible, φ holds |
| `[<+A>] φ` | **Committed**: must do A, and φ holds after |

## The Diamondbox `[<+A>]`

The diamondbox operator `[<+A>]` means:
- The agent CAN perform action A
- The agent CANNOT refuse action A
- After A, the formula φ holds

With several labels, `[<+A +B>] φ` commits to all of them: no commit may
move without A, and none without B.

This is the key operator for expressing commitments.

## Temporal Operators (Sugar)

| Operator | Meaning | Definition |
|----------|---------|------------|
| `always(φ)` | φ holds now and forever | `gfp(X, φ & []X)` |
| `eventually(φ)` | φ holds now, or some path of moves reaches φ | `lfp(X, φ \| <>X)` |
| `until(p, q)` | some path keeps p true until it reaches q | `lfp(X, q \| (p & <>X))` |

`eventually` and `until` promise that φ can be reached, not that it will be.
No rule can make a commit happen.

## Fixed Points

For complex properties, use fixed points directly:

```modality
// Greatest fixed point (νX) - invariants, safety
gfp(X, property & []X)

// Least fixed point (μX) - reachability, liveness
lfp(X, target | <>X)
```

## Boolean Connectives

```modality
φ & ψ           // Conjunction (and)
φ | ψ           // Disjunction (or)
!φ              // Negation (not)
!φ | ψ          // Conditional rule: if φ, then ψ
true            // Always true
false           // Always false
```

Prefer explicit Boolean conditionals in examples. The parser may accept
formula-level implication sugar, but `!φ | ψ` keeps temporal steps visually
separate from proof implication.

## State Predicates in Formulas

Rules constrain *who can commit* based on contract state. Use path-based predicates:

```modality
// Check a boolean flag
bool_true(/status/delivered.bool)

// Check text value
text_eq(/status.text, "delivered")

// Every RELEASE commit needs the buyer's signature
always([+RELEASE -signed_by(/parties/buyer.id)] false)

// Committed to sign (diamondbox with predicate)
[<+signed_by(/parties/seller.id)>] true
```

Rules gate *who* can commit and which labelled transitions are allowed based on
state, signatures, and other evidence.
