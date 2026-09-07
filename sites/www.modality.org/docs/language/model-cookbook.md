---
sidebar_position: 7
title: Model Cookbook
---

# Model Cookbook

Read this before writing a witness model for a rule. Do not search `rust/` or
synthesizer tests for examples.

A witness is a small labeled transition system that **shows the rule is
possible**. Nodes are opaque (`q0`, `q1`, …). Edge labels carry the meaning.

## Bootstrap vs later steps

When the rule is `[] always(...)`, the **first** step is unconstrained. Keep
an unlabeled edge out of the initial node, then put the rule's labels on later
edges.

```
q0 --> q1
q1 --> …
```

A signed self-loop on `q0` would force the bootstrap commit to be signed.

## After this commit either Alice or Bob must sign

Rule:

```
[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
```

Witness: unlabeled bootstrap, then Alice **or** Bob on the steady state.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
```

## After this commit Alice and Bob alternate

Rule: every later step is signed by Alice or Bob, and the same party cannot
sign twice in a row.

Witness: unlabeled bootstrap, then a two-state cycle. No same-signer self-loop.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id)
    q2 --> q1: +signed_by(/parties/bob.id)
  }
}
```
