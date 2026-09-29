---
sidebar_position: 2
title: Predicate Theory
---

# Predicate Theory

A rule or a model edge names predicates: `+signed_by(/parties/alice.id)`,
`+num_gt(/balance.num, "100")`, `-modifies(/members)`. The predicate theory
is what the validator knows about how these labels relate. It answers two
questions:

- **Can these labels hold on one commit?** `+num_gt(/x.num, "5")` and
  `+num_lt(/x.num, "3")` cannot, so an edge that carries both is **dead**:
  no commit takes it.
- **Do these labels imply that one?** A commit with `+num_gt(/x.num, "7")`
  also has `+num_gt(/x.num, "5")`, so an edge labelled with the first
  meets a rule that asks for the second.

These facts follow from the rules and predicates the contract already has.
The validator derives them when it checks a commit; nobody writes them down,
and they are never stored in the log.

## Versions

Each network fixes one predicate theory version from genesis
(`predicate_theory_version` in the network parameters). Every validator on
the network judges every commit with it, so every node reaches the same
verdict.

| Version | What it knows |
|---------|---------------|
| `v0` | Labels are names. A label and its negation contradict; nothing else does. The default, and what the testnet runs. |
| `v1` | Refused on networks: it accepts rules some runs break. Kept for local previews (`--theory v1`). |
| `v2` | Everything below. |

`modal c commit` previews `v2` on every commit, and `modal c theory` shows
what `v2` derives for a contract, whatever the network runs.

## What `v2` knows

**Sorts.** Each standard predicate reads one kind of fact:

- **order:** `num_eq`, `num_gt`, `num_gte`, `num_lt`, `num_lte` and
  `amount_in_range`, compared exactly over decimals, against literals or
  other `.num` paths. So `x > 5`, `y < 3` and `y > x` cannot all hold;
- **signers:** `signed_by`, `any_signed`, `all_signed` and `threshold`,
  counted over the keys at a path in accepted state. So
  `+threshold("2", /treasury)` needs two keys under `/treasury`;
- **paths:** `modifies`, `post_to_path` and `sets`, over the path tree.
  So `+post_to_path(/a/b)` implies `+modifies(/a)`;
- **text:** `text_eq` and its substring forms;
- **methods:** `+POST`, `+MODEL`, `+RULE` and the other method labels.

**Wrong kinds.** A predicate whose arguments are the wrong kind never holds:
`num_gt(/x.text, "5")`, `num_gt(/x.num, "five")`, `signed_by(/x.text)`.

**Predicates the validator does not evaluate** never hold: `before`,
`after`, `timestamp_valid`, `hash_matches`, `+wasm(...)` and any name missing
from [Standard Predicates](standard-predicates.md). An edge that needs one is
dead.

**External predicates.** `oracle_attests` holds when the commit carries the
attestation. The commit can attach it or not without changing anything else,
so it is a free choice, but only if the oracle's key is in accepted state.

**Opaque predicates.** `sent_eq`, `sent_lte`, `sent_to`, `emitted_by` and
`keeps_product` are evaluated, but the theory knows nothing more about them. On an edge each may
hold or not; it contradicts only its own negation. A box that forbids one
therefore sees every edge that might take it, which is sound. A diamond that
needs one is refused: the theory cannot show that any commit takes it.
`posts_own_key(/p.id)` is known only to post to `/p.id`.

**Accepted state.** The theory reads what earlier commits wrote. If
`/x.num` is `3`, an edge needing `+num_gt(/x.num, "5")` is dead there. What an
edge's `-modifies(/p)` leaves unchanged carries to the next step.

**Signatures.** Every signature on a commit must verify, or the commit is
refused (see
[Commit signatures](standard-predicates.md#commit-signatures)).

## How a rule is checked

A rule is anchored at the commit that adds it. It is checked on the model
from the states that commit reaches, with the accepted state of that commit.
Every later commit must stay inside a model that still meets it.

- **A box**, `[+X -E] false`, ranges over every commit that could take an
  edge with those labels. An edge counts unless the theory shows the two
  label sets cannot hold together.
- **A diamond**, `<+X> true`, needs a commit that takes an edge with those
  labels. An edge counts only if the theory shows some commit carries both
  the edge's labels and the diamond's. When the theory cannot tell, the edge
  does not count.
- **After the first step**, a diamond counts only an edge every run at that
  node can take. A run may have written anything the edges allow, so `v2`
  refuses some true rules rather than accept a false one.

`v0` matches labels by name. An edge that does not mention `+L` counts for
`<+L> true`, and a dead edge counts like any other.

## When the theory cannot tell

The theory answers yes, no or unknown. The validator acts only on a definite
answer: an edge is dead only when the theory says no commit takes it, and a
diamond counts an edge only when the theory says some commit does. So an
unknown can make `v2` refuse a model that is fine, but never accept one that
breaks a rule. Cases the theory leaves unknown today include:

- keys that a `-threshold` bound forces apart;
- numbers outside the exact decimal range;
- a text needle that starts with `/`;
- `sets` of one path to two different values, which the theory does not yet
  see clash;
- custom `+wasm(...)` predicates, which never hold until the validator
  evaluates them.

## What it does not promise

A model that meets every rule does not show the contract can always move.
`always([] false)` is met by a model with no moves, and then every later
commit is refused. `modal c theory` lists dead edges and each move out of the
current state as open, blocked or forced. Read it before signing a rule.

## Related

- [Standard Predicates](standard-predicates.md): what each predicate reads
- [Rule Syntax](../language/rule-syntax.md): rules, anchoring and lint findings
- [`modal c theory`](../cli/contract-commands.md#theory): the derived view of a contract
