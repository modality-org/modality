---
sidebar_position: 4
title: A Move That Never Happens
---

# A Move That Never Happens

Three people share a contract. Any one of them may post a note. Changing
who the members are takes all three. A member writing the member list alone
is a move that never happens.

This is the safety shape: after every step of a certain kind, a fact still
holds. Here the fact is "this step did not both change `/members` and lack
someone's signature," which the box writes as a forbidden step.

Create one more identity beside Alice and Bob. In this contract Alice's key
stands in as Carol, Bob's as Dave, and the new one as Erin. The paths are
what the rules name.

```bash
modal id create --name example/erin
modal contract create --dir ./members
cd members
modal checkout
modal set-named-id /members/carol.id example/alice
modal set-named-id /members/dave.id example/bob
modal set-named-id /members/erin.id example/erin
modal add-rule --name signed 'always([-any_signed(/members)] false)'
modal add-rule --name membership \
  'always([+modifies(/members) -all_signed(/members)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +modifies(/members) +all_signed(/members)
  }
}
```

```bash
modal commit --all -m "Members"
modal commit --path /notes.text --value hello --sign example/alice -m "Carol posts a note"
```

The note lands. One signature was enough, and the note is not under
`/members`.

Add a fourth member, Frank, with only Carol's signature.

```bash
modal id create --name example/frank
modal set-named-id /members/frank.id example/frank
modal commit --all --sign example/alice -m "Carol adds Frank"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q1 [+any_signed(/members) -modifies(/members)]; failed predicates: forbidden -modifies(/members) matched
```

The other edge out of `q1` needs `all_signed(/members)`. Sign with all three
keys that are already members. Frank's key is not a member until this
commit is accepted, so his signature does not count toward the three.

```bash
modal commit --all --sign example/alice --sign example/bob --sign example/erin -m "All add Frank"
```

That lands. `/members/frank.id` is in the log.

## The idea

`always([+modifies(/members) -all_signed(/members)] false)` says every step
that changes `/members` without every member's signature is forbidden. The
box ranges over those steps. The witness keeps a step for ordinary writes,
marked `-modifies(/members)`, and a different step for membership, marked
with `all_signed`. A later witness that lets one member change the list is
refused, because the rule is still there. `all_signed` and `any_signed` are
in [standard predicates](../reference/standard-predicates). The same shape
shows up when a grade says a draft was too weak or too strong
([the rule suite](../reference/ai-rule-suite)).

Next: [A finish that stays possible](finish-stays-possible).
