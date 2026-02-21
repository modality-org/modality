# How Contract Validation Works

**The governing model's transition predicates are the SOLE enforcement mechanism.**

This is the most important thing to understand about Modality validation. If you see code that evaluates rules directly against commits, it is wrong.

## The Protocol

1. A **MODEL** commit defines a state machine with predicate-guarded transitions
2. Every subsequent commit must match a valid transition from the current state
3. Each transition has predicates: `+pred` must hold, `-pred` must NOT hold
4. **Rules are NEVER evaluated directly at commit time**
5. Rules only constrain which models are acceptable as witnesses
6. No model → no constraints (commits pass freely)

## Example

```modality
model hello_world {
  initial active
  active -> active [+any_signed(/) -modifies(/README.md) -adds_rule]
}
```

This model has one transition from `active → active` with three predicates:
- `+any_signed(/)` — commit must have at least one signature
- `-modifies(/README.md)` — commit must NOT write to `/README.md`
- `-adds_rule` — commit must NOT be a RULE commit

Every incoming commit is checked against this transition. If no transition's predicates are fully satisfied, the commit is rejected.

## Rules vs Models

- **Rules** = permanent protection formulas (e.g. `always (+any_signed(/))`)
- **Models** = replaceable state machines with predicate-guarded transitions
- When adding a RULE, you must provide a **witness model** that proves the rule is satisfiable
- The model can be replaced later, but only with one that still satisfies all accumulated rules

## Predicates

| Predicate | Meaning |
|-----------|---------|
| `+signed_by(/path.id)` | Commit signed by the key at that state path |
| `+any_signed(/path)` | Commit signed by any member under path |
| `+all_signed(/path)` | Commit signed by ALL members under path |
| `+modifies(/path)` | Commit writes to path or subpath |
| `+adds_rule` | Commit is a RULE method |
| `+threshold(n, /path)` | At least n members under path signed |

Prefix `+` means "must hold". Prefix `-` means "must NOT hold".

## Validation Flow (for implementors)

```
1. Replay existing commits to get current model + model state + data state
2. For each new commit:
   a. If no model loaded → accept (no constraints)
   b. Find all transitions from current model state
   c. For each candidate transition, check ALL predicates against the commit
   d. If any transition fully matches → accept, advance state to transition.to
   e. If NO transition matches → reject
3. After validation, apply commit to data state (POST updates paths, etc.)
```

**Do NOT evaluate rules directly. Do NOT build a separate rule evaluator. The model handles everything.**
