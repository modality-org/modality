# Tutorial 2: Combining Predicates

This tutorial explains how predicates interact and how Modality detects contradictions.

## The Problem: Conflicting Rules

Imagine two parties adding rules to a contract:

**Alice adds:**
```modality
rule alice_wants_short {
  formula { text_length_lt(/data/msg.text, 10) }
}
```

**Bob adds:**
```modality  
rule bob_wants_long {
  formula { text_length_gt(/data/msg.text, 20) }
}
```

These rules **contradict** each other! No string can be both shorter than 10 AND longer than 20.

## How Correlate Detects Contradictions

When predicates are combined, `correlate` analyzes their interaction:

```json
// text_length_lt(10) correlating with text_length_gt(20)
{
  "formulas": [
    "!(text_length_lt($path, 10) & text_length_gt($path, 20))"
  ],
  "satisfiable": false,
  "gas_used": 15
}
```

The output tells us:
- `satisfiable: false` → these rules cannot both be true
- The formula `!(A & B)` explicitly states the contradiction

## Compatible Rules

Not all combinations conflict. Consider:

**Alice adds:**
```modality
rule must_be_greeting {
  formula { text_equals(/data/msg.text, "hello") }
}
```

**Bob adds:**
```modality
rule must_be_5_chars {
  formula { text_length_eq(/data/msg.text, 5) }
}
```

Correlate output:
```json
{
  "formulas": [
    "text_equals($path, \"hello\") -> text_length_eq($path, 5)"
  ],
  "satisfiable": true,
  "gas_used": 20
}
```

These are **compatible** because `"hello"` has exactly 5 characters. The formula `A -> B` means A implies B - if the text equals "hello", it will automatically satisfy the length requirement.

## Constraining Rules

Some rules don't contradict but add constraints:

**Rule 1:** `text_starts_with("foo")`
**Rule 2:** `text_ends_with("bar")`

```json
{
  "formulas": [
    "text_starts_with($path, \"foo\") & text_ends_with($path, \"bar\") -> text_length_gt($path, 5)"
  ],
  "satisfiable": true,
  "gas_used": 15
}
```

The combined rules imply the text must be at least 6 characters (to fit both "foo" and "bar").

## Real Example: User Registration

Consider a username validation contract:

```modality
// Username must not be empty
rule username_required {
  formula { text_not_empty(/user/name.text) }
}

// Username must be reasonable length
rule username_length {
  formula {
    text_length_gt(/user/name.text, 2) &
    text_length_lt(/user/name.text, 20)
  }
}

// Username must start with letter (simplified)
rule username_format {
  formula { text_starts_with(/user/name.text, "user_") }
}
```

Running correlate on these:
- `text_not_empty` + `text_length_gt(2)` → compatible (gt implies not_empty)
- `text_starts_with("user_")` + `text_length_gt(2)` → compatible ("user_" is 5 chars)
- `text_starts_with("user_")` + `text_length_lt(20)` → compatible (prefix fits)

All rules are satisfiable together!

## Boolean Combinations

Bool predicates follow similar logic:

```modality
rule must_agree {
  formula { bool_is_true(/terms/agreed.bool) }
}

rule explicit_consent {
  formula { bool_equals(/terms/agreed.bool, true) }
}
```

Correlate:
```json
{
  "formulas": [
    "bool_is_true($path) <-> bool_equals($path, true)"
  ],
  "satisfiable": true,
  "gas_used": 10
}
```

The `<->` means these are **equivalent** - they express the same constraint.

## Detecting Problems Early

The key benefit of correlate: **catch contradictions before runtime**.

Instead of waiting for a transaction to fail, you can:
1. Collect all rules for a path
2. Run correlate on each predicate
3. Check if `satisfiable` is false anywhere
4. Reject contradictory rule combinations upfront

## Summary

| Relationship | Formula | satisfiable |
|--------------|---------|-------------|
| Compatible | `A -> B` | true |
| Equivalent | `A <-> B` | true |
| Constrains | `A & B -> C` | true |
| Contradiction | `!(A & B)` | false |

## Next Steps

- [Tutorial 3: Contract Validation](./03-contract-validation.md) - Put it all together in a real contract
