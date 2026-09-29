---
sidebar_position: 3
title: Contract Evolution
---

# Contract Evolution

Modality contracts evolve by appending commits. They do not edit old terms in
place.

The useful mental model is:

- `POST` commits change contract state.
- `RULE` commits add accumulated constraints.
- `MODEL` commits replace the witness model. The commit is judged by the
  candidate model it posts, which must replay the accepted history and meet
  every accumulated rule. The old model is not consulted.

Rules are the authority. Models are witnesses that show the accumulated rules
remain satisfiable and provide the transition predicates used to accept or
reject the next commit.

## V1: An Open Bootstrap

A minimal first contract can start with a bootstrap transition and then move to
a governed steady state:

```modality
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST +signed_by(/parties/alice.id)]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
```

The initial setup commit is accepted because it can take `q0 -> q1` with both
`+POST` and `+MODEL`. After replay reaches `q1`, an unsigned `POST` is rejected
because the only `+POST` successor requires Alice's signature.

## V2: Add a Rule, Then Replace the Witness

To make the post-bootstrap protection survive model replacement, append a rule
commit:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
```

This does not remove the old model. It adds a permanent constraint over future
witness models: every commit after the one that adds it must include either
Alice's or Bob's signature. The `+RULE` transition above is what
allows this separate rule commit; without it, the old model would reject the
rule addition before the accumulated rule set can grow.

A later `MODEL` commit is judged by the candidate model it posts, not by the
old one. It is accepted only if:

- the candidate model can replay the accepted history, and the commit takes
  one of its edges from the state that history reaches;
- the candidate model satisfies every accumulated rule.

So the rules, not the model, are what protect the contract. Anyone may post a
candidate model; only the rules decide which candidates pass.

This replacement is acceptable because every steady-state successor remains
signed:

```modality
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST +signed_by(/parties/alice.id)]
  q1 -> q1 [+POST +signed_by(/parties/bob.id)]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
```

This replacement is rejected because it exposes an unsigned steady-state
successor:

```modality
export default model {
  initial q0

  q0 -> q1 [+POST +MODEL]
  q1 -> q1 [+POST]
  q1 -> q1 [+RULE +signed_by(/parties/alice.id)]
  q1 -> q1 [+MODEL +signed_by(/parties/alice.id)]
}
```

The important point is that replacement is not mutation. It is a new commit,
judged by the candidate model and by every accumulated rule.

## Protected Party Changes

Party changes should be ordinary state changes guarded by the current model:

```modality
export default model {
  initial active

  active -> active [+POST +any_signed(/members) -modifies(/members)]
  active -> active [+POST +modifies(/members) +all_signed(/members)]
  active -> active [+MODEL +all_signed(/members)]
}
```

The first transition admits non-membership updates with one member signature.
The second transition admits membership edits only when all accepted
`/members/*.id` identities sign. The third transition asks the same authority
for witness replacement, but a model cannot protect itself: a replacement is
judged by the model it posts, so a candidate without that transition is judged
without it. To make replacement need every member, add a rule:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([+MODEL -all_signed(/members)] false)
  }
}
```

The contract evolution CLI smoke runs this pattern end to end: Alice alone can
append an ordinary note, Alice can add Bob while she is the only accepted member,
Bob can then append an ordinary note, Alice alone cannot post a replacement that keeps the all-members `+MODEL` transition after Bob is accepted, Alice and Bob together can replace the witness model with repeated `--sign` flags, and Alice alone cannot add `/members/carol.id` after Bob is accepted. Both one-signer rejected commits report `missing +all_signed(/members)`.
The smoke checks both JSON and human-readable status/log output, so the visible
CLI view still shows `Model state: active`, the accepted evolution messages, and
the signer IDs after replacement.

## Bounded Terms

Terms that should expire need explicit language support. A rule may not name a
model node such as `active` or `expired`: node names are the model author's
choice and bind no commit, so posted rules that use them are refused. State the
term with labels. The contract CLI smoke covers a small bounded term:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([-signed_by(/parties/alice.id) -bool_true(/terms/delivery_complete.bool)] false)
  }
}
```

Until accepted state has `/terms/delivery_complete.bool` set to true, every
commit needs Alice's signature; after that, anyone may post. In that smoke, the
current model keeps signed ordinary updates in the `active` state, moves to
`expired` only after previously accepted `/terms/delivery_complete.bool`
evidence satisfies `+bool_true(/terms/delivery_complete.bool)`, and then allows
unsigned ordinary updates in `expired`, each carrying the same label. The run
proves the bounded term by appending the bounded rule,
rejecting the guarded move before completion evidence, accepting the same move
after completion evidence, and ending replay in the `expired` witness state.

The same smoke also appends an older rule first, that completion stays
possible:

```modality
export default rule {
  starting_at $PARENT
  formula {
    eventually(<+bool_true(/terms/delivery_complete.bool)> true)
  }
}
```

It then rejects a later witness replacement whose every move carries
`-bool_true(/terms/delivery_complete.bool)`, so completion can never happen.
That keeps the evolution lesson honest: bounded terms can make a commitment
expire, but unrelated older rules still accumulate and continue constraining
replacement models. `eventually` promises a path, not a commit: under predicate
theory `v0` a replacement that simply drops the completion edge, without
forbidding the label, still passes; `v2` refuses it. A parser-only `until(...)`
example is useful language evidence, but it is not contract
evolution evidence by itself.
The smoke also checks the human-readable bounded-term status/log view, including
`Model state: expired`, so the CLI surface a developer reads matches the replay
state proven by JSON.
