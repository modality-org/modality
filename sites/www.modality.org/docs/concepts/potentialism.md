---
sidebar_position: 6
title: Potentialism
---

# Potentialist State Machines

The deepest concept: a contract is not a fixed labeled transition system — it's one **actualization** from a space of **potential** labeled transition systems.

## The Insight

When Alice and Bob start a contract:

1. **Potential is infinite** — Any labeled transition system is possible
2. **Alice adds a rule** — Potential shrinks to labeled transition systems satisfying her rule
3. **Bob adds a rule** — Potential shrinks further
4. **Rules accumulate** — The space of valid futures only contracts, never expands

## The Monotonicity Theorem

> Adding a covenant (rule) can only shrink the space of valid extensions, never expand it.

This is why Modality contracts are safe: each party adds their protection, and no one can add a rule that invalidates existing protections.

## Visualizing Potential

```
Initial: All possible labeled transition systems
    │
    ▼
┌─────────────────────────────────┐
│  ∞ potential machines           │
│                                 │
│  Alice's rule: "always safe"    │
│           │                     │
│           ▼                     │
│  ┌─────────────────────────┐    │
│  │ Smaller set satisfying   │   │
│  │ Alice's constraint       │   │
│  │                          │   │
│  │  Bob's rule: "fair"      │   │
│  │         │                │   │
│  │         ▼                │   │
│  │  ┌─────────────────┐     │   │
│  │  │ Even smaller set │    │   │
│  │  │ satisfying both  │    │   │
│  │  └─────────────────┘     │   │
│  └─────────────────────────┘    │
└─────────────────────────────────┘
```

## Why This Matters

Traditional contracts are static: you agree to terms, and that's it.

Modality contracts are **dynamic**: 
- Start with nothing
- Each party adds their protections
- The contract evolves within the constrained space
- No party can violate accumulated protections

This is ideal for **agent cooperation** where trust must be established incrementally through verifiable commitments, not assumed upfront.

## Further Reading

See the [Potentialist LTS Paper](/docs/resources/potentialist-lts) for the formal treatment.
