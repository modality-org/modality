---
sidebar_position: 4
title: Predicates
---

# Predicates Reference

This page names the language vocabulary. The currently verified local
first-contract path is narrower: method labels, `signed_by`, `any_signed`,
`all_signed`, `threshold`, `modifies`, `post_to_path`, `has_property`,
`state_exists`, `text_eq`, `text_contains`, `text_starts_with`,
`text_ends_with`, `amount_in_range`, `num_eq`, `num_gt`, `num_gte`, `num_lt`,
`num_lte`, `bool_true`, and `bool_false` are enforced from
replayable commit artifacts. See the
[standard predicate evidence matrix](../reference/standard-predicates.md) for
the exact source of each fact.

Do not treat the future vocabulary below as runtime evidence until a validator
path documents its artifact format and tests. Oracle, time, hash, and most
WASM predicates are extension vocabulary in the local first-contract path, not
proof that external facts were checked.

## Signature Predicates

```modality
+signed_by(/path/to/identity.id)
// Commit must be signed by this ed25519 key

+threshold("2", /signers/list)
// At least two signatures from the list required

+signed_by_n(/signers/list, n)
// At least n signatures from the list
```

## Oracle Predicates

```modality
+oracle_attests(/oracle.id, "condition")
// Oracle attests to a condition

+oracle_attests_fresh(/oracle.id, "condition", max_age_seconds)
// Oracle attestation within time limit
```

## Time Predicates

The validator does not evaluate time, hash, or `wasm` predicates yet. They
never hold, so no commit takes a transition that needs one; see
[Standard Predicates](../reference/standard-predicates.md#implementation-status).

```modality
+before(/deadlines/cutoff.datetime)
// Current time is before deadline

+after(/deadlines/start.datetime)
// Current time is after start time
```

## Comparison Predicates

```modality
// Numeric comparisons
+num_eq(/path/a.num, /path/b.num)     // a == b
+num_gt(/path/a.num, /path/b.num)     // a > b
+num_gte(/path/a.num, /path/b.num)    // a >= b
+num_lt(/path/a.num, /path/b.num)     // a < b
+num_lte(/path/a.num, /path/b.num)    // a <= b

// Text comparisons
+text_eq(/path/a.text, /path/b.text)
+text_contains(/path/a.text, "substring")
+text_starts_with(/path/a.text, "prefix")
+text_ends_with(/path/a.text, "suffix")

// Accepted-state numeric range
+amount_in_range(/invoice/amount.num, "10", "100")
+amount_in_range(/invoice/amount.num, /limits/min.num, /limits/max.num)

// Boolean
+bool_true(/path/flag.bool)
+bool_false(/path/flag.bool)

// Accepted-state path existence
+state_exists(/path/ready.flag)
```

The local validator derives `state_exists` from accepted-state path existence
only; a path written by the same pending commit is not evidence for that commit.

The local validator derives `bool_true` and `bool_false` from accepted-state
booleans only; a boolean written by the same pending commit is not evidence for
that commit.

The local validator also derives `num_eq`, `num_gt`, `num_gte`, `num_lt`, and
`num_lte` from accepted-state numbers only. The right-hand side can be a
literal number or another accepted-state number path; a number written by the
same pending commit is not evidence for that commit.

A number argument needs no quotes: `+num_gte(/x.num, 100)` is the same
predicate as `+num_gte(/x.num, "100")`, and `-3` and `0.25` work too.

A fraction such as `1/4` or `2/6` is a number argument too. One with an exact
decimal is written as that decimal, so `1/4` is the same predicate as `0.25`
and works under any predicate theory. Any other fraction is reduced (`2/6` is
`1/3`) and is a number from predicate theory `v4`; before `v4` a predicate
given one never holds. `1/0` is a parse error.

From predicate theory `v5` a number path can also be `.rational`, `.integer`
or `.natural` ([Path types](path-types.md#numbers)):
`+num_lt(/out/i.natural, /out/pad.natural)` compares two whole numbers.

The local validator derives `text_eq`, `text_contains`, `text_starts_with`, and
`text_ends_with` from accepted-state strings only. `text_eq` can compare two
accepted-state string paths or one path to a literal; the other text predicates
check literal substrings. Text written by the same pending commit is not
evidence for that commit.

## Hash Predicates

```modality
+hash_matches(/commitments/secret.hash, /revealed/value.text)
// SHA256 of revealed value matches stored hash
```

## Custom WASM Predicates

```modality
+wasm(/predicates/my_predicate.wasm, arg1, arg2)
// Call custom WASM predicate with arguments
```
