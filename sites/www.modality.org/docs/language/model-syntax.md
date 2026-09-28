---
sidebar_position: 2
title: Model Syntax
---

# Model Syntax

Models define labeled transition systems. Nodes are opaque witness worlds; edge
labels and predicates carry the contract meaning.

## Basic Structure

```modality
model <name> {
  initial <node>

  <from_node> -> <to_node> [+<label> +<predicate1> -<predicate2>]
}
```

## Complete Example

```modality
model escrow_witness {
  initial q0

  q0 -> q1 [+DEPOSIT +signed_by(/parties/buyer.id) +num_gte(/deposit/amount.num, /terms/price.num)]
  q1 -> q2 [+DELIVER +signed_by(/parties/seller.id)]
  q2 -> q3 [+RELEASE +signed_by(/parties/buyer.id)]
  q1 -> q4 [+DISPUTE +signed_by(/parties/buyer.id)]
}
```

The names `q0`, `q1`, `q2`, and so on are not escrow phases. They identify
nodes in a witness LTS so the verifier can evaluate modal formulas.

## Nodes

```modality
initial q0
```

`initial` names the starting witness node. Additional nodes are introduced by
transitions. Prefer neutral names such as `q0`, `q1`, or generated ids unless
you are writing a low-level debugging example.

## Transitions

```modality
// Basic labeled transition
q0 -> q1 [+ACTION_NAME]

// With predicates
q1 -> q2 [+ACTION_NAME +predicate1(args) -predicate2(args)]

// Multiple transitions with the same label are nondeterministic
q1 -> q2 [+DISPUTE]
q1 -> q3 [+DISPUTE +signed_by(/parties/arbiter.id)]
```

## Parts

```modality
model Contract {
  part a {
    q0 --> q1: +signed_by(/parties/alice.id)
  }
  part b {
    q0 --> q2: +signed_by(/parties/bob.id)
  }
}
```

Parts are one graph joined by node name. A commit at `q0` may take an edge
out of `q0` in any part, so rules are checked against every part's edges
together: `[-signed_by(/parties/alice.id)] false` fails here, because part `b`
lets Bob commit alone. A node name in two parts is the same node.

A part of same-node loops allows commits that do not advance the rest of the
model, such as notes or evidence, while the contract stays at its node:

```modality
model Contract {
  part flow {
    q0 --> q1: +signed_by(/parties/alice.id)
    q1 --> q2: +signed_by(/parties/bob.id)
  }
  part notes {
    q1 --> q1: +signed_by(/parties/alice.id) -modifies(/parties)
  }
}
```

Rules range over these loops like any other edge, so give them the signer
labels the rules require. A loop keeps the contract at `q1` but may still write
state; `-modifies` says which paths it leaves unchanged.

## Comments

```modality
// Single-line comment

/*
   Multi-line
   comment
*/

model witness {
  initial q0
  q0 -> q1 [+START]
}
```
