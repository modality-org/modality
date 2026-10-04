---
sidebar_position: 6
title: The Rule That Looks Right
---

# The Rule That Looks Right

You want an invoice to change only after a memo already says `paid`. A
formula can look like that and still be the wrong shape. The box is the
shape that forbids the invoice. The diamond is the shape that talks about
some step existing.

## The formula that looks right

```bash
modal contract create --dir ./memo
cd memo
modal checkout
modal set-named-id /parties/alice.id example/alice
modal add-rule --name looks \
  'always(!<+modifies(/invoice)> true | <+modifies(/invoice) +text_eq(/memo.text, "paid")> true)'
```

`modal add-rule` writes the file and warns:

```output
modality/guarded-diamond: this says only that some `+modifies(/invoice)` move carries the evidence where a `+modifies(/invoice)` move exists; a model can hold another `+modifies(/invoice)` move without it, and a commit may take that move
forbid the move without the evidence: `always([+modifies(/invoice) -text_eq(/memo.text, paid)] false)`
```

A witness with a free signed step beside a careful one does not satisfy that
formula. Write this and try to commit it.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/alice.id) +modifies(/invoice) +text_eq(/memo.text, "paid")
  }
}
```

```bash
modal commit --all -m "Looks right"
```

```output
Model violates rule 'local_rule'
formula: always(!<+modifies(/invoice)> true | <+modifies(/invoice) +text_eq(/memo.text, "paid")> true)
counterexample: always unexpectedly failed
```

The formula did not say "every invoice change carries the memo." It said
something the checker will not accept this witness for. Delete
`rules/looks.modality` before you continue. A rule file left in `rules/`
is part of the next commit.

## The box

`text_eq` reads the memo already in the log. A memo written by the same
commit is invisible to it. The memo lives at `/memo.text`, outside
`/invoice`, so posting it leaves the invoice untouched.

```bash
modal add-rule --name memo \
  'always([+modifies(/invoice) -text_eq(/memo.text, "paid")] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) +post_to_path(/memo.text) -modifies(/invoice)
    q2 --> q2: +signed_by(/parties/alice.id) +modifies(/invoice) +text_eq(/memo.text, "paid")
    q2 --> q2: +signed_by(/parties/alice.id) -modifies(/invoice)
  }
}
```

```bash
modal commit --all -m "Memo rule"
modal commit \
  --path /invoice/amount.text \
  --value "10 units" \
  --sign example/alice \
  -m "Too soon"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q2 [+signed_by(/parties/alice.id) +post_to_path(/memo.text) -modifies(/invoice)]; failed predicates: forbidden -modifies(/invoice) matched, missing +post_to_path(/memo.text)
```

Post the memo, then the invoice.

```bash
modal commit --path /memo.text --value paid --sign example/alice -m "Memo"
modal commit --path /invoice/amount.text --value "10 units" --sign example/alice -m "Invoice"
```

Both land. The second one is an invoice change, and the accepted memo
already says `paid`.

## The idea

`[+modifies(/invoice) -text_eq(/memo.text, "paid")] false` says every step
that changes `/invoice` while the memo is missing is forbidden. That is a
box: it ranges over matching steps. `<+modifies(/invoice) …> true` says
some such step exists. A box of `true` holds when no step matches, so it
does not show that the step happened. Lint names that
`modality/vacuous-box-guard` and `modality/guarded-diamond`. The operators
are in [modal logic](../concepts/modal-logic). `text_eq` reads accepted
state, as [standard predicates](../reference/standard-predicates) describes.

Next: [A key, not a person's name](whose-turn).
