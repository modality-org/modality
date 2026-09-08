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

## Skip-current vs from-now

- `[] φ` constrains **successors** of the current state. Use it when the
  request says "after this commit", "from the next commit", or "later commits".
- Plain `always(φ)` also constrains the **current** step. Do not use it for
  "after this commit".

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
| After this commit either Alice or Bob must sign | `[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)` |
| After this commit Alice must sign | `[] always([-signed_by(/parties/alice.id)] false)` |
| After this commit either Alice or Bob must sign in alternating turns | `[] always(([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false))` |
| Named action `X` requires Alice's signature | `always(!<+X> true \| <+X +signed_by(/parties/alice.id)> true)` |

Use the named-action row **only** when the user named an action. The
alternating-turns row still requires a signer on every later step, then forbids
the same party twice in a row.

## Anti-patterns

```
always(!<+SIGN> true | (<+SIGN +signed_by(/parties/alice.id)> true | <+SIGN +signed_by(/parties/bob.id)> true))
```

That invents `+SIGN`, does not skip the bootstrap commit, and should not be
used unless the contract actually has a `+SIGN` transition. When a named
action is real, bind required signature evidence to the same transition label
with explicit Boolean form.

Do not prefix formulas with `F1:`. Do not wrap them in markdown fences.
