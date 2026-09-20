---
sidebar_position: 4
title: Predicates
---

# Predicates Reference

This page names the language vocabulary. The currently verified local
first-contract path is narrower: method labels, `signed_by`, `any_signed`,
`all_signed`, `threshold`, `modifies`, `post_to_path`, `has_property`,
`state_exists`, `text_eq`, `amount_in_range`, `num_eq`, `num_gt`, `num_gte`,
`num_lt`, `num_lte`, `bool_true`, and `bool_false` are enforced from
replayable commit artifacts. See the
[standard predicate evidence matrix](../reference/standard-predicates.md) for
the exact source of each fact.

Do not treat the future vocabulary below as runtime evidence until a validator
path documents its artifact format and tests. Oracle, time, text-contains,
hash, and most WASM predicates are extension vocabulary in the local
first-contract path, not proof that external facts were checked.

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
