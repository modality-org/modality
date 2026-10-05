---
sidebar_position: 3
title: States, Not One Room
---

# States, Not One Room

An offer does not count because someone wishes it did. Alice posts the
terms. Bob accepts them after that. A commit that accepts first has no step
to take.

You already know a witness is a few states and labeled edges. This page
makes the edges a sequence. Identities from
[the previous page](rule-that-stays) are reused.

```bash
modal contract create --dir ./offer
cd offer
modal checkout
modal set-named-id /parties/alice.id example/alice
modal set-named-id /parties/bob.id example/bob
modal add-rule --name authorized \
  'always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) +post_to_path(/offer/terms.text)
    q2 --> q3: +signed_by(/parties/bob.id) +post_to_path(/offer/accepted.text) +state_exists(/offer/terms.text)
    q3 --> q3: +signed_by(/parties/alice.id) -modifies(/offer)
  }
}
```

Write that to `model/default.modality` and commit it with the rule and the
two identities.

```bash
modal commit --all -m "Offer machine"
```

You are in `q1`. The only step out of `q1` is Alice posting the terms. Bob
tries to accept now.

```bash
modal commit \
  --path /offer/accepted.text \
  --value yes \
  --sign example/bob \
  -m "Accept early"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q2 [+signed_by(/parties/alice.id) +post_to_path(/offer/terms.text)]; failed predicates: missing +post_to_path(/offer/terms.text), missing +signed_by(/parties/alice.id)
```

The accept edge exists in the witness. Replay is not standing on it. Post
the terms, then accept.

```bash
modal commit --path /offer/terms.text --value "10 units" --sign example/alice -m "Offer"
modal commit --path /offer/accepted.text --value yes --sign example/bob -m "Accept"
modal status
```

```output
Model state: q3
```

## The idea

The witness is the machine of possible steps. A commit is accepted when it
is one of those steps from the state replay has reached. Bob's signature
satisfied the rule and still had nothing to step to, because the terms were
not posted yet. See [models and rules](../concepts/models-vs-rules) and the
[model cookbook](../language/model-cookbook).

Next: [A move that never happens](members-only-contract).
