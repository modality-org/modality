---
sidebar_position: 2
title: Potentialist LTS Paper
---

# Potentialist State Machines and Labeled Transition Systems

:::info
Full paper available at [GitHub](https://github.com/modality-org/modality/blob/main/docs/papers/POTENTIALIST-LTS.md)
:::

## Abstract

This paper presents a novel approach to contract verification based on **potentialist metaphysics** applied to labeled transition systems. Rather than treating contracts as fixed state machines, we model them as actualizations from a space of potential state machines, constrained by an append-only list of covenants (temporal modal formulas).

## Core Thesis

> The current model isn't a fixed state machine — it's one *actualization* of a space of potential state machines. Covenants constrain which actualizations are valid.

## Labeled Transition Systems (LTS)

An LTS is a tuple `(S, Λ, →, s₀)` where:
- `S` is a set of states
- `Λ` is a set of action labels
- `→ ⊆ S × Λ × S` is the transition relation
- `s₀ ∈ S` is the initial state

## Potentialist Extension

A **Potentialist LTS** (P-LTS) is a tuple `P = (L, C)` where:
- `L` is the current (actual) LTS
- `C = [c₁, c₂, ..., cₙ]` is an ordered list of covenants (modal formulas)

## Valid Extensions

`Pot(P)` denotes the space of valid potential extensions:

```
Pot(P) = { L' | L' ⊇ L ∧ ∀c ∈ C: L' ⊨ c }
```

## Monotonicity Theorem

> **Theorem**: Adding a covenant can only shrink the space of valid extensions, never expand it.

```
C' = C ∪ {cₙ₊₁} ⟹ Pot(L, C') ⊆ Pot(L, C)
```

This is the key safety property: each party's protection rules can only constrain, never enable, new behaviors.

## Covenants as Modal Formulas

Covenants are expressed in modal mu-calculus with hybrid extensions:

- **Box**: `[a]φ` — after all a-transitions, φ holds
- **Diamond**: `<a>φ` — after some a-transition, φ holds
- **Diamondbox**: `[<+a>]φ` — committed to a, and φ holds after
- **Fixed points**: `μX.φ`, `νX.φ` — least/greatest fixed points
- **Anchoring**: `starting_at <commit>` — binds formula to a point in history

## Model Witnesses

Every rule must provide an explicit model (LTS) demonstrating satisfiability. This prevents:
- Vacuous promises
- Contradictory rule sets
- Unsatisfiable contracts

## Applications to Agent Cooperation

This framework is ideal for AI agent cooperation because:

1. **Incremental trust** — Parties add protections one at a time
2. **Formal verification** — Commitments are mathematically checkable
3. **No central authority** — Constraints are self-enforcing
4. **Composability** — Rules from different parties combine safely

## Further Reading

- Linnebo & Studd — *Generality and the Foundations of Set Theory*
- Hamkins — *The Modal Logic of Set-Theoretic Potentialism*
- Stirling — *Modal and Temporal Properties of Processes*
