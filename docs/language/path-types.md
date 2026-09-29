---
sidebar_position: 5
title: Path Types
---

# Path Syntax

Paths reference data in the contract state:

```
/directory/subdirectory/file.type
```

## Path Types

| Extension | Type | Example |
|-----------|------|---------|
| `.id` | ed25519 public key | `/parties/alice.id` |
| `.num` | Numeric value | `/terms/price.num` |
| `.text` | Text string | `/metadata/description.text` |
| `.bool` | Boolean | `/flags/approved.bool` |
| `.datetime` | ISO 8601 timestamp | `/deadlines/expiry.datetime` |
| `.date` | Date (YYYY-MM-DD) | `/terms/start.date` |
| `.hash` | SHA256 hash | `/commitments/secret.hash` |
| `.json` | JSON data | `/config/settings.json` |
| `.wasm` | WASM module | `/predicates/custom.wasm` |
| `.modality` | Model/rule file | `/model/default.modality` |

## Examples

```modality
// Reference a party's identity
+signed_by(/parties/alice.id)

// Compare numeric values
+num_gte(/deposit/amount.num, /terms/price.num)

// Check a timestamp
+after(/deadlines/expiry.datetime)

// Verify a hash commitment
+hash_matches(/commitments/secret.hash, /revealed/value.text)
```

## Variables

A path segment `$k` is a variable, and `$k.id` is that variable followed by
an extension. A variable stands for one segment with no dot: `alice`, not
`alice.id`.

- In a **rule**, a variable means every name.
  `always([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false)`
  says that for every `k`, no later commit writes under `/claimants/k`
  unless the key at `/claimants/k.id` signed it.
- On a **model edge**, a variable means one name the commit picks. A commit
  takes `+signed_by(/claimants/$k.id)` when some claimant's accepted key
  signed it. Every `$k` on one edge is the same name.

A segment `!$k` is a **hole**: every segment outside `$k`'s slot. Holes go on
model edges only. `-modifies(/claimants/!$k)` forbids a write under
`/claimants/bob`, `/claimants/bob.id`, or `/claimants/bob.bool` for every
name `bob` other than `k`. `!$k.id` is the `.id` of every other name.

```modality
// Some claimant signs, and the commit writes in her slot only
+signed_by(/claimants/$k.id) -modifies(/claimants/!$k)

// No other claimant signs
-signed_by(/claimants/!$k.id)
```

Limits:

- Variables go only in the path arguments of standard predicates that read
  paths: `signed_by`, `any_signed`, `all_signed`, `threshold`, `modifies`,
  `post_to_path`, `state_exists`, `bool_true`, `bool_false`, `text_eq`,
  `text_contains`, `text_starts_with`, `text_ends_with`, `num_eq`, `num_gt`,
  `num_gte`, `num_lt`, `num_lte`, `amount_in_range`, `sets` (also spelled
  `post_to`), `sent_eq`, `sent_lte`, `sent_to`, and `posts_own_key`.
- A hole with no extension (`!$k`) goes only in `modifies`, `post_to_path`,
  `state_exists`, `any_signed`, `all_signed`, and `threshold`. Elsewhere,
  write the extension: `-bool_true(/claimants/!$k.bool)`.
- A rule takes no holes. Its variables already range over every name.
- Labels read accepted state. `+signed_by(/claimants/$k.id)` needs the key
  at `/claimants/k.id` before the commit. The commit that registers a key
  cannot also count as signed by it. To require that the registered key
  signed its own registration, use `+posts_own_key(/claimants/$k.id)`,
  which reads the pending commit.

## Directory Structure

A typical contract has this structure:

```
my-contract/
├── .contract/           # Internal storage
├── state/               # Data files
│   ├── parties/
│   │   ├── alice.id
│   │   └── bob.id
│   ├── terms/
│   │   └── price.num
│   └── deadlines/
│       └── expiry.datetime
├── model/
│   └── default.modality
├── rules/
│   └── protection.modality
├── alice.passfile
└── bob.passfile
```
