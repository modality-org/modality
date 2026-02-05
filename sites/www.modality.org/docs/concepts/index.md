---
sidebar_position: 1
title: Overview
---

# Core Concepts

This guide explains the fundamental concepts behind Modality.

## The Five Pillars

| Concept | Purpose |
|---------|---------|
| **Append-only log** | Immutable, ordered history |
| **State machines** | Finite, verifiable behaviors |
| **Modal logic** | Express temporal constraints |
| **Predicates** | Bind identity to logic |
| **Potentialism** | Safe accumulation of rules |

## How They Fit Together

1. A **contract** is an append-only log of signed commits
2. Commits can add **state**, **rules**, or perform **actions**
3. A **model** (labeled transition system) defines what actions are possible
4. **Rules** (temporal formulas) constrain which evolutions are valid
5. **Predicates** bind real-world identity (signatures) to logical constraints
6. **Potentialism** ensures rules can only shrink the space of valid futures

Dive into each concept:

- [Append-Only Logs](./append-only-logs)
- [State Machines](./state-machines)
- [Modal Logic](./modal-logic)
- [Predicates](./predicates)
- [Potentialism](./potentialism)
