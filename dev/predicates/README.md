# Modality Predicates

Predicates are the building blocks for contract rules. Each predicate has two functions:

- **evaluate(context, params)** → checks if the predicate holds for given data
- **correlate(params, other_rules)** → generates formulas expressing interactions with other predicates

## Available Predicates

### Text Predicates (for `.text` paths)

| Predicate | Description | Parameters |
|-----------|-------------|------------|
| `text_equals` | Exact string match | `expected: string` |
| `text_equals_ignore_case` | Case-insensitive match | `expected: string` |
| `text_contains` | Substring check | `substring: string` |
| `text_starts_with` | Prefix check | `prefix: string` |
| `text_ends_with` | Suffix check | `suffix: string` |
| `text_is_empty` | Check if empty | (none) |
| `text_not_empty` | Check if not empty | (none) |
| `text_length_eq` | Exact length | `length: number` |
| `text_length_gt` | Length greater than | `length: number` |
| `text_length_lt` | Length less than | `length: number` |

### Bool Predicates (for `.bool` paths)

| Predicate | Description | Parameters |
|-----------|-------------|------------|
| `bool_is_true` | Check if true | (none) |
| `bool_is_false` | Check if false | (none) |
| `bool_equals` | Check equals value | `expected: bool` |
| `bool_not` | Check is NOT value | `of: bool` |

### Number Predicates (for numeric values)

| Predicate | Description | Parameters |
|-----------|-------------|------------|
| `num_equals` | Exact match (epsilon) | `expected: number` |
| `num_gt` | Greater than | `threshold: number` |
| `num_lt` | Less than | `threshold: number` |
| `num_gte` | Greater than or equal | `threshold: number` |
| `num_lte` | Less than or equal | `threshold: number` |
| `num_between` | In range (exclusive) | `min: number, max: number` |
| `num_positive` | Check > 0 | (none) |
| `num_negative` | Check < 0 | (none) |
| `num_zero` | Check == 0 | (none) |

### Timestamp Predicates (for temporal constraints)

| Predicate | Description | Parameters |
|-----------|-------------|------------|
| `timestamp_before` | Before deadline | `deadline: i64` |
| `timestamp_after` | After deadline | `deadline: i64` |
| `timestamp_within` | In time window | `start: i64, end: i64` |
| `timestamp_expired` | Deadline passed | `deadline: i64, current: i64` |
| `timestamp_near` | Within tolerance | `target: i64, tolerance: i64` |

### Hash Predicates (for commitment schemes)

| Predicate | Description | Parameters |
|-----------|-------------|------------|
| `sha256_matches` | SHA-256 verification | `data: hex, expected_hash: hex` |
| `hash_equals` | Compare hashes | `hash1: hex, hash2: hex` |
| `commitment_verify` | Commitment scheme | `preimage: hex, salt: hex, commitment: hex` |
| `hash_format` | Valid hash format | `hash: hex, algorithm: string` |

## How Correlate Works

When multiple predicates apply to the same path, `correlate` generates formulas that express their logical relationship:

```
// Compatible rules generate implications
text_equals("hello") + text_length_eq(5)
→ "text_equals($path, \"hello\") -> text_length_eq($path, 5)"

// Contradictory rules generate negated conjunctions  
text_equals("hello") + text_length_eq(10)
→ "!(text_equals($path, \"hello\") & text_length_eq($path, 10))"

// Equivalent rules generate biconditionals
bool_is_true + bool_equals(true)
→ "bool_is_true($path) <-> bool_equals($path, true)"
```

## Examples

See the [examples](./examples/) directory for complete working examples.

## Tutorials

- [Basic Predicate Usage](./tutorials/01-basic-predicates.md)
- [Combining Predicates](./tutorials/02-combining-predicates.md)
- [Contract Validation](./tutorials/03-contract-validation.md)
