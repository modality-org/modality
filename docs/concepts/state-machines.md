---
sidebar_position: 3
title: Labeled Transition Witnesses
---

# Labeled Transition Witnesses

A Modality model is a labeled transition system (LTS): anonymous witness nodes
connected by labeled transitions. The contract meaning lives on the transition
labels and predicates, not in business-named states.

## Basic Structure

```modality
model escrow_witness {
  initial q0

  q0 -> q1 [+DEPOSIT +signed_by(/parties/buyer.id)]
  q1 -> q2 [+DELIVER +signed_by(/parties/seller.id)]
  q2 -> q3 [+RELEASE +signed_by(/parties/buyer.id)]
  q1 -> q4 [+REFUND +signed_by(/parties/seller.id)]
}
```

The node ids (`q0`, `q1`, ...) are witness-world names. They are useful for
model checking and counterexamples, but they should not be treated as contract
phases like "funded" or "released."

## Components

- `initial` identifies the starting witness node.
- `q0 -> q1 [...]` declares an edge in the witness LTS.
- `+DEPOSIT`, `+DELIVER`, and `+RELEASE` are transition labels.
- `+signed_by(...)` and similar terms are predicates that must hold for the
  transition to be usable.

## Visualization

The model above is best read as labeled edges:

```text
q0 --[+DEPOSIT +signed_by(buyer)]--> q1
q1 --[+DELIVER +signed_by(seller)]--> q2
q2 --[+RELEASE +signed_by(buyer)]--> q3
q1 --[+REFUND +signed_by(seller)]--> q4
```

## Why Opaque Nodes?

Opaque witness nodes keep the core model evolvable. A future contract version
can add new labels, predicates, branches, or obligations without baking a
domain-specific phase taxonomy into the semantics.

User-facing tools may project parts of the LTS into friendly descriptions, but
those descriptions are views. The contract remains a set of formulas over
labeled transitions.
