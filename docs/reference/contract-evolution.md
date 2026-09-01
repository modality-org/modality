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
- `MODEL` commits replace the witness model only when the old accepted model
  allows a `+MODEL` transition and the candidate model can still replay the
  accepted history.

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
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
```

This does not remove the old model. It adds a permanent constraint over future
witness models: after the rule's parent point, every reachable successor must
include either Alice's or Bob's signature. The `+RULE` transition above is what
allows this separate rule commit; without it, the old model would reject the
rule addition before the accumulated rule set can grow.

A later `MODEL` commit is accepted only if both checks pass:

- The old accepted model has a matching `+MODEL` transition from the current
  witness state, including Alice's signature.
- The candidate model satisfies the accumulated rule and can replay the
  accepted history.

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

The important point is that replacement is not mutation. It is a new commit
checked by the old model and by every accumulated rule.

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
`/members/*.id` identities sign. The third transition makes witness replacement
use the same membership authority.

The contract evolution CLI smoke runs this pattern end to end: Alice alone can
append an ordinary note, Alice can add Bob while she is the only accepted member,
Bob can then append an ordinary note, Alice alone cannot replace the witness model after Bob is accepted, Alice and Bob together can replace the witness model with repeated `--sign` flags, and Alice alone cannot add `/members/carol.id` after Bob is accepted. Both one-signer rejected commits report `missing +all_signed(/members)`.

## Bounded Terms

Terms that should expire need explicit language support, such as an
`until`-bounded formula, before they are safe to teach as mutable commitments.
Until that path is runnable in the contract CLI, examples should say that old
rules keep accumulating and should not imply that a later rule deletes an older
one.

Do not promote a bounded-term example into onboarding until a contract CLI smoke
can append the bounded rule, show the guarded move rejected before the boundary,
show the same move accepted after the explicit completion or expiry evidence,
and still show that unrelated older rules remain accumulated. A parser-only
`until(...)` example is useful language evidence, but it is not contract
evolution evidence by itself.
