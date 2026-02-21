---
name: modality-contracts
description: Build, validate, and interact with Modality contracts — append-only logs of signed commits governed by predicate-guarded state machine models. Use when implementing contract validators, hub services, CLI contract commands, contract APIs, or writing/reviewing any code that touches Modality contract commit processing, rule enforcement, or model validation.
---

# Modality Contracts

## Core Protocol

A contract is an **append-only log of signed commits**. Two things govern it:

- **Model** — a replaceable state machine with predicate-guarded transitions
- **Rules** — permanent protection formulas (can never be removed)

### The One Rule

**The governing model's transition predicates are the SOLE enforcement mechanism.**

- Every commit must match a valid transition from the current model state
- Each transition has predicates: `+pred` must hold, `-pred` must NOT hold
- **Rules are NEVER evaluated directly at commit time**
- Rules only constrain which models are acceptable witnesses
- No model → no constraints (commits pass freely)

**If you see code evaluating rules directly against commits, it is wrong.**

## Model Syntax

```modality
model name {
  initial state_name
  state_a -> state_b [+pred1(arg) -pred2(arg)]
  state_b -> state_b []  // permissive self-loop
}
```

Bare `[]` = no predicates (any commit matches). Omit brackets entirely for the same effect.

## Predicates

| Predicate | Holds when |
|-----------|-----------|
| `signed_by(/path.id)` | Commit signed by key stored at state path |
| `any_signed(/path)` | Commit signed by any `.id` member under path |
| `all_signed(/path)` | Commit signed by ALL `.id` members under path |
| `modifies(/path)` | Commit writes to path or any subpath |
| `adds_rule` | Commit method is RULE (no parens needed) |
| `threshold(n, /path)` | At least n members under path signed |

Prefix `+` = must hold. Prefix `-` = must NOT hold.

**Key gotcha:** Without `-modifies(/members)` on a general transition, it could be used to modify membership with a single signature. Always pair path-sensitive transitions with negative guards on unrelated transitions.

## Rule Syntax

```modality
rule name {
  formula {
    always (+any_signed(/))
  }
}
```

Rules use `always()`, `eventually()`, `until()` — temporal operators that are sugar for modal mu-calculus fixed points. Rules constrain which models are valid, not commits directly.

## MODEL Replacement Must Satisfy All Rules

A MODEL commit is validated against every accumulated rule. Each transition in the candidate model must enforce every rule predicate. You cannot replace a restrictive model with a permissive one.

## Rule Commits Require a Witness Model

When submitting a RULE commit, include a model that proves satisfiability:

```bash
modal c commit --method rule \
  --rule 'rule X { formula { ... } }' \
  --model 'model Y { initial s; s -> s [] }' \
  --sign key
```

Without a satisfying model → rejected. Prevents deadlock from unsatisfiable rules.

## Commit Methods

| Method | Purpose |
|--------|---------|
| `POST` | Write data to a state path |
| `DELETE` | Remove a state path |
| `MODEL` | Set/replace the governing model |
| `RULE` | Add a permanent rule (needs witness) |
| `REPOST` | Cross-contract data reference |
| `CREATE` | Create asset |
| `SEND` / `RECV` | Asset transfer |

## Validation Flow (for implementors)

```
1. Replay existing commits → build current model, model state, data state
2. For each new commit:
   a. No model loaded → accept (no constraints)
   b. Find transitions from current model state
   c. For each candidate transition, check ALL predicates against commit
   d. If any transition fully matches → accept, advance to transition.to
   e. If NO transition matches → reject with predicate failure details
3. Apply commit to data state (POST updates paths, MODEL replaces model, etc.)
```

## Canonical Example: Members-Only Contract

```modality
model members_only {
  initial active
  active -> active [+any_signed(/members) -modifies(/members)]
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

First transition: any member can sign, but CAN'T touch `/members`. Second: CAN modify `/members`, but needs ALL signatures. Without `-modifies(/members)` on the first, a single member could change membership.

## State

- State = key-value map built by replaying POST/DELETE commits
- Member keys stored as `/members/name.id` → pubkey hex
- Signatures extracted from commit `head.signatures`, `signature` field, or `signatures` array

## Reference

For full syntax details: read `references/validation-flow.md`
For predicate implementation details: read `references/predicates.md`
