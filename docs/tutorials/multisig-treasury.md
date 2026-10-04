---
sidebar_position: 8
title: Two of Three
---

# Two of Three

A note in the treasury contract can be one holder's word. A spend takes two
of the three. One holder moving the funds is the refusal.

`threshold("2", /treasury)` counts distinct keys posted under `/treasury`.
The same key written in two spellings is one signer. Rules compare keys.
See [Modality IDs](../concepts/modality-ids).

```bash
modal id create --name example/erin
modal contract create --dir ./treasury
cd treasury
modal checkout
modal set-named-id /treasury/alice.id example/alice
modal set-named-id /treasury/bob.id example/bob
modal set-named-id /treasury/carol.id example/erin
modal add-rule --name signed 'always([-any_signed(/treasury)] false)'
modal add-rule --name spend \
  'always([+modifies(/treasury) -threshold("2", /treasury)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/treasury) -modifies(/treasury)
    q1 --> q1: +threshold("2", /treasury) +modifies(/treasury)
  }
}
```

```bash
modal commit --all -m "Treasury"
modal commit --path /notes.text --value hello --sign example/alice -m "A note"
modal commit \
  --path /treasury/spend.text \
  --value "pay the vendor" \
  --sign example/alice \
  -m "One holder"
```

The note lands. The spend does not.

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q1 [+any_signed(/treasury) -modifies(/treasury)]; failed predicates: forbidden -modifies(/treasury) matched
```

The spend edge is the other one. It wanted two signatures. The refusal also
names it: `missing +threshold("2", /treasury)`, one authorized signature of
two, from three members. Sign with a second holder. Two signatures on one
commit are `modal c commit --all --sign bob --sign carol` when those are the
identity names. With the names above:

```bash
modal commit --all \
  --sign example/alice \
  --sign example/bob \
  --path /treasury/spend.text \
  --value "pay the vendor" \
  -m "Two holders"
```

`--path` on that command posts the spend in the same commit as the
signatures. If `/treasury/spend.text` is already in the working tree from
the refused attempt, `modal commit --all --sign example/alice --sign example/bob`
is the same spend. It lands.

## The idea

`threshold("n", /path)` holds when at least `n` distinct keys posted under
that path signed this commit. The box
`always([+modifies(/treasury) -threshold("2", /treasury)] false)` forbids a
change under `/treasury` that arrived with fewer than two. The witness
still has a one-signature step, and that step is marked
`-modifies(/treasury)`, so a note keeps working. Threshold is a
[standard predicate](../reference/standard-predicates).

Next: [Pay someone](pay-someone).
