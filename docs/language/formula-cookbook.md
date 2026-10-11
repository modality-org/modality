---
sidebar_position: 6
title: Formula Cookbook
---

# Formula Cookbook

Read this before writing or suggesting a Modality rule formula. Do not search
`rust/`, `experiments/`, or `node_modules` for examples.

Reply with **one formula**: the inner contents of a rule `formula { ... }`
block, suitable as the argument to `modal add-rule`. No markdown, no `F1:`
labels, no explanation.

## Where a rule starts

A rule is checked from the state the commit that adds it reaches. The
commit that adds the rule is never constrained by it.

- `always(φ)` constrains **every commit after this one**. Use it when the
  request says "after this commit", "from the next commit", or "later commits".
- Do not prefix it with `[]`. `[] always(φ)` also leaves the **next** commit
  free, and any stranger may use that commit to replace the model and the
  keys the rule names.

## Do not invent actions

Do not invent names such as `+SIGN`, `+COMMIT`, `+UPDATE`, or `+ALICE_TURN`.
If the request does not name a specific action, constrain **signatures** on
every remaining step.

- `[-signed_by(A)] false` — a step that lacks A's signature is forbidden.
- `[-signed_by(A) -signed_by(B)] false` — a step that lacks **both** is
  forbidden (either A or B may sign).

Prefer identity paths from the request or the contract. For Alice and Bob in a
typical first contract that is `/parties/alice.id` and `/parties/bob.id`.

## Recipes

| Requirement | Formula |
|-------------|---------|
| After this commit either Alice or Bob must sign | `always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)` |
| After this commit Alice must sign | `always([-signed_by(/parties/alice.id)] false)` |
| After this commit either Alice or Bob must sign in alternating turns | `always(([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false))` |
| Named action `X` requires Alice's signature | `always([+X -signed_by(/parties/alice.id)] false)` |
| After this commit nothing under a claimant's slot changes without that claimant's key (any number of claimants) | `always([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false)` |

Use the named-action row **only** when the user named an action. The
alternating-turns row still requires a signer on every later step, then forbids
the same party twice in a row.

## Possibility and liveness

The recipes above are boxes: `[L] false` forbids every step with labels `L`.
A diamond says a step is possible: `<L> φ` holds when some step with labels `L`
leads to a state where `φ` holds. `eventually` and `until` are built from
diamonds.

These rules are about the witness model you post. They promise a step **can**
still happen, not that anyone takes it: no rule makes a commit happen. Post a
model that has the edges the rule asks for (see
[model-cookbook.md](model-cookbook.md)).

| Requirement | Formula |
|-------------|---------|
| From this commit, the next step can be one Bob signs | `<+signed_by(/parties/bob.id)> true` |
| From this commit, a step Bob signs can still be reached | `eventually(<+signed_by(/parties/bob.id)> true)` |
| After this commit, Bob can always sign the next step | `always(<+signed_by(/parties/bob.id)> true)` |
| After this commit, a step Bob signs always stays reachable | `always(eventually(<+signed_by(/parties/bob.id)> true))` |
| Either Alice or Bob must sign every later step, and a step Bob signs can still be reached | `always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & eventually(<+signed_by(/parties/bob.id)> true)` |

## Until

`until` has two readings. Write the one the requirement means.

- `p until q`, or `eventually(p) until q`: **some** path keeps `p` true until
  it reaches `q`. A possibility, like `eventually`.
- `always(p) until q`: on **every** path, `p` holds until `q` does. `q` need
  never come: a run where `p` holds forever meets it. This is the safety form.

`until(p, q)` is another way to write `p until q`.

| Requirement | Formula |
|-------------|---------|
| From this commit, Alice can keep signing until a step Bob signs is possible | `<+signed_by(/parties/alice.id)> true until <+signed_by(/parties/bob.id)> true` |
| From this commit, no step writes `/payout` until a payout every party signs is possible | `always([+modifies(/payout)] false) until <+modifies(/payout) +all_signed(/parties)> true` |

`p until q` holds from the state the rule starts at. To make `always(p) until q`
hold again after `q` has come, wrap it: `always(always(p) until q)`.

Two things decide whether a model meets these rules:

- A promise that something stays possible needs the model to protect what it
  depends on. Bob can sign later only while his key is still at
  `/parties/bob.id`. A model whose edges let a commit change `/parties` cannot
  promise that, so `always(<+signed_by(/parties/bob.id)> true)` is refused
  until every edge says `-modifies(/parties)`.
- Labels constrain a step; they do not list everything it may do. An edge
  `+signed_by(/parties/alice.id)` also allows a step Bob co-signs, so
  `<+signed_by(/parties/bob.id)> true` holds there. To rule Bob out, the edge
  must say `-signed_by(/parties/bob.id)`.

## Fixed points

Every temporal form above is a fixed point. `gfp(X, φ)` is the largest set of
states where `φ` holds when `X` stands for that set: what holds forever.
`lfp(X, φ)` is the smallest: what can be reached in finitely many steps. Inside
a fixed point, `X` is a variable it binds, never a model node.

```
always(φ)          = gfp(X, φ & []X)
eventually(φ)      = lfp(X, φ | <>X)
p until q          = lfp(X, q | (p & <>X))
always(p) until q  = gfp(X, q | (p & []X))
```

The recipes above, written out:

| Requirement | Formula |
|-------------|---------|
| After this commit either Alice or Bob must sign | `gfp(X, [-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false & []X)` |
| From this commit, a step Bob signs can still be reached | `lfp(X, <+signed_by(/parties/bob.id)> true or <>X)` |

The payout rule from [Until](#until), written out:

```
gfp(X, <+modifies(/payout) +all_signed(/parties)> true or (([+modifies(/payout)] false) & []X))
```

In every state from here: a payout every party signs is possible, or no step
writes `/payout` and the same holds after every step.

Nest fixed points to say what the temporal forms cannot. Some run keeps coming
back to a step Bob signs, forever:

```
gfp(X, lfp(Y, <+signed_by(/parties/bob.id)> X or <>Y))
```

`or` and `|` are the same.

## Anti-patterns

```
always(!<+SIGN> true | (<+SIGN +signed_by(/parties/alice.id)> true | <+SIGN +signed_by(/parties/bob.id)> true))
```

That invents `+SIGN`, and should not be used unless the contract actually has
a `+SIGN` transition. It is also too weak: it holds when a `+SIGN` move with a
signature exists beside one without. When a named action is real, forbid the
move without the evidence: `always([+SIGN -signed_by(/parties/alice.id)] false)`.

Do not put a hole (`!$k`) in a rule. In a rule, `$k` already means every
name. Holes are for model edges (see
[variables](path-types.md#variables)).

Do not prefix formulas with `F1:`. Do not wrap them in markdown fences.
