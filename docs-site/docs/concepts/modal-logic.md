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

This is the key operator for expressing commitments.

## Temporal Operators (Sugar)

| Operator | Meaning | Definition |
|----------|---------|------------|
| `always φ` | φ holds now and forever | `gfp(X, φ & []X)` |
| `eventually φ` | φ holds now or sometime later | `lfp(X, φ \| <>X)` |
| `until(p, q)` | p holds until q becomes true | `lfp(X, q \| (p & <>X))` |

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
φ -> ψ          // Implication
true            // Always true
false           // Always false
```
