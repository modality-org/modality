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
Each turn also excludes the other signer.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) -signed_by(/parties/bob.id)
    q2 --> q1: +signed_by(/parties/bob.id) -signed_by(/parties/alice.id)
  }
}
```

Without `-signed_by(/parties/alice.id)` on Bob's turn, a commit Alice and Bob
both sign takes that edge, and Alice can sign again right after. The rule is
refused for that model.

## Any number of claimants, each in their own slot

Rule: after this commit, nothing under a claimant's slot changes without that
claimant's key, and a registered key is replaced only by its holder.

```
[] always(([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false) & ([+modifies(/claimants/$k.id) +state_exists(/claimants/$k.id) -signed_by(/claimants/$k.id)] false))
```

Witness: unlabeled bootstrap, then three kinds of step. A step outside the
registry. A new claimant registers a fresh `.id` and writes nothing else under
`/claimants`. A claimant signs and writes in their own slot only.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: -modifies(/claimants)
    q1 --> q1: -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
    q1 --> q1: +signed_by(/claimants/$k.id) -modifies(/claimants/!$k)
  }
}
```

`$k` is a name each commit picks, and `!$k` is every other claimant's slot
(see [variables](path-types.md#variables)). Without
`-modifies(/claimants/!$k)` on the last edge, Alice can sign a commit that
writes into Bob's slot, and the rule is refused for that model. One commit
writes one slot, so co-signers write their slots in separate commits.
