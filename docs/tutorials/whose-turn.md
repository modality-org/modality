---
sidebar_position: 7
title: A Key, Not a Person's Name
---

# A Key, Not a Person's Name

Alice's turn, then Bob's. The contract does not have an action called
Alice. It has a key posted at `/parties/alice.id`, and a commit signed by
that key. The same person twice in a row is the refusal.

The rule and the witness are the alternating-turns recipe in the
[formula cookbook](../language/formula-cookbook) and the
[model cookbook](../language/model-cookbook).

```bash
modal contract create --dir ./turns
cd turns
modal checkout
modal set-named-id /parties/alice.id example/alice
modal set-named-id /parties/bob.id example/bob
modal add-rule --name turns \
  'always(([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false))'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) -signed_by(/parties/bob.id)
    q2 --> q1: +signed_by(/parties/bob.id) -signed_by(/parties/alice.id)
  }
}
```

```bash
modal commit --all -m "Turns"
modal commit --path /move.text --value alice --sign example/alice -m "Alice"
modal commit --path /move.text --value "alice again" --sign example/alice -m "Alice again"
```

```output
No valid transition for local commit from current states {"q2"}
Closest candidate transition: q2 -> q1 [+signed_by(/parties/bob.id) -signed_by(/parties/alice.id)]; failed predicates: forbidden -signed_by(/parties/alice.id) matched, missing +signed_by(/parties/bob.id)
```

Bob's key is the step that exists from here.

```bash
modal commit --path /move.text --value bob --sign example/bob -m "Bob"
```

That lands, and the next turn is Alice's again.

## The idea

`signed_by(/parties/alice.id)` holds when the pending commit is signed by
the key already stored at that path. The label on the edge is that
predicate. A commit both of them sign matches
Bob's edge unless the edge also says `-signed_by(/parties/alice.id)`, which
is why the cookbook witness excludes the other signer. Paths and keys are
in [Modality IDs](../concepts/modality-ids).

Next: [Two of three](multisig-treasury).
