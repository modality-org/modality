---
sidebar_position: 1
title: Standard Predicates
---

# Standard Predicates

Predicates are the building blocks for governing-model transitions and contract
rules. The local contract-log validator enforces the predicates below from
replayable commit artifacts: the pending commit body, the pending commit
signatures, and the already accepted contract state.

## Current Local Evidence Matrix

These are the predicate and label facts currently used by the first-contract
local validator path.

| Fact | Evidence source | Current-state rule |
|------|-----------------|--------------------|
| `+POST`, `+MODEL`, and other method labels | Pending commit body methods | Checked on the pending commit |
| `signed_by(/path.id)` | Pending commit signatures plus the public key string at `/path.id` in accepted state | Reads previously committed state, not values written by the same commit |
| `any_signed(/path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | At least one listed identity must sign |
| `all_signed(/path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | The path must contain at least one identity, and every listed identity must sign |
| `threshold("n", /path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | At least `n` unique listed identities must sign |
| `modifies(/path)` | Pending commit body paths | Matches `/path` itself or descendants such as `/path/alice.id` |
| `post_to_path(/path)` | Pending commit body methods and paths | Matches a `POST` action to `/path` itself or a descendant |
| `has_property(/path, "a.b")` | Accepted-state JSON at `/path` | Reads previously committed JSON and follows dot-separated object keys |

Other reference predicates below describe the intended standard vocabulary.
Treat them as requiring predicate-specific implementation and tests before
using them in the local first-contract path.

## Implementation Status

Use this table to distinguish the predicate vocabulary from the predicates
currently enforced by the local first-contract validator.

| Predicate family | Local first-contract validator | Notes |
|------------------|--------------------------------|-------|
| Method labels such as `+POST` and `+MODEL` | Enforced | Derived from pending commit body methods |
| `signed_by`, `any_signed`, `all_signed`, `threshold`, `modifies`, `post_to_path`, `has_property` | Enforced | Derived from pending signatures, accepted state, pending methods, pending paths, and accepted-state JSON |
| `timestamp_valid` | Unit-tested extension module only | Implemented in `modal-wasm-validation`; not yet replay evidence for the local first-contract validator |
| `before`, `after`, state predicates, hash predicates, `oracle_attests`, and `wasm` | Not first-contract-local yet | Intended extension vocabulary; treat as external or future predicate checks unless a validator path explicitly documents support |

## Path Predicates

### modifies

Checks if the commit writes to a path itself or a descendant path.

```modality
+modifies(/members)
```

**Arguments:**
- `path` — Path or ancestor path to check

**Behavior:**
- Returns true if any path in the commit body is the path itself or a descendant
- Does not match sibling paths that merely share a string prefix
- Used for path-based access control rules

**Example:**
```modality
// Only allow membership changes if all members sign
always(![+modifies(/members)] true | <+all_signed(/members)> true)
```

### post_to_path

Checks if the pending commit includes a `POST` action to the path itself or a
descendant path.

```modality
+post_to_path(/config)
```

**Arguments:**
- `path` — Path or ancestor path to check

**Behavior:**
- Looks only at the pending commit body
- Ignores non-`POST` actions, even when they write under the same path
- Returns true if any `POST` action targets the path itself or a descendant
- Does not match sibling paths that merely share a string prefix

## Signature Predicates

### signed_by

Verifies the commit is signed by a specific ed25519 key.

```modality
+signed_by(/users/alice.id)
```

**Arguments:**
- `path` — Path to the public key in contract state

**Behavior:**
- Looks up the public key string at `path` in the accepted contract state
- Passes if the pending commit includes a matching signature
- Does not see identity files written by the same pending commit

### any_signed

Verifies at least one member from a path has signed.

```modality
+any_signed(/members)
```

**Arguments:**
- `path` — Path or ancestor path containing member public keys

**Behavior:**
- Enumerates all `.id` files at the path or descendants
- Does not count identities from sibling paths that merely share a string prefix
- Passes if ANY member has a valid signature
- Used for "any member can act" patterns

### all_signed

Verifies ALL members from a path have signed.

```modality
+all_signed(/members)
```

**Arguments:**
- `path` — Path or ancestor path containing member public keys

**Behavior:**
- Enumerates all `.id` files at the path or descendants
- Does not count identities from sibling paths that merely share a string prefix
- Passes only if EVERY member has a valid signature
- Fails when the path contains no `.id` members
- Used for "unanimous consent" patterns like adding members

### threshold

Verifies n-of-m signatures from the accepted identities under a path.

```modality
+threshold("2", /treasury/signers)
```

**Arguments:**
- `n` — Minimum signatures required
- `signers_path` — Path or ancestor path containing signer public keys in `*.id` files

**Behavior:**
- Enumerates all `.id` files at the path or descendants in accepted contract state
- Does not count identities from sibling paths that merely share a string prefix
- Counts each authorized public key at most once
- Ignores commit signatures from keys that are not listed under the path
- Passes when at least `n` unique listed identities signed the pending commit
- Rejection output reports the authorized signature count, accepted member count,
  missing signature count, and any unauthorized signatures that were ignored

## Time Predicates

The `timestamp_valid` extension module compares an input timestamp with the
predicate context timestamp in unit tests. It is still external to the local
first-contract path because replay must define where the trusted clock value
comes from before deadline predicates can be treated as verifier evidence.

### before

Intended predicate for checking that current time is before a deadline.

```modality
before(/deadlines/expiry.datetime)
```

### after

Intended predicate for checking that current time is after a timestamp.

```modality
after(/deadlines/start.datetime)
```

## State Predicates

The local validator now derives `has_property(/path, "a.b")` from accepted
contract state. It looks up the previously committed JSON value at `/path` and
follows dot-separated object keys such as `a.b`. It does not see JSON written
by the same pending commit.

```modality
has_property(/profiles/alice.json, "contact.email")
```

The `modal-wasm-validation` crate also has unit-tested state-inspection modules,
but only the `has_property` binding above is first-contract-local replay
evidence today. Treat other state predicate inputs as explicit JSON
predicate-test data until a contract-log validator path documents how the JSON
is derived from replayed commits and accepted state.

### bool_true / bool_false

Intended predicates for checking boolean state values.

```modality
bool_true(/status/delivered.bool)
bool_false(/flags/cancelled.bool)
```

### text_eq

Intended predicate for comparing text values.

```modality
text_eq(/status.text, "approved")
```

### num_eq / num_gt / num_gte / num_lt / num_lte

Intended predicates for numeric comparisons.

```modality
num_gte(/balance.num, 100)
num_lt(/deposit.num, /limit.num)
```

## Oracle Predicates

### oracle_attests

Intended predicate for checking a signed attestation from a trusted oracle.
This is external evidence vocabulary until a validator path documents the
attestation format, freshness rule, replay binding, and signature check.

```modality
oracle_attests(/oracles/delivery.id, "delivered", "true")
```

**Arguments:**
- `oracle_path` — Path to oracle's public key
- `claim` — The claim type being attested
- `value` — Expected value (optional)

**Security features:**
- Should verify oracle signatures
- Should enforce attestation freshness
- Should bind attestations to a specific contract
- Should prevent replay attacks

## Hash Predicates

### hash_matches

Intended predicate for checking a SHA256 hash commitment.

```modality
hash_matches(/commitments/secret.hash, /revealed/value.text)
```

## Using Predicates in Rules

Predicates are combined with logical operators in rule formulas:

```modality
export default rule {
  starting_at $PARENT
  formula {
    // All commits must be signed by alice OR bob
    always(<+signed_by(/users/alice.id)> true | <+signed_by(/users/bob.id)> true)
  }
}

export default rule {
  starting_at $PARENT
  formula {
    // After deadline, only buyer can commit
    always(!<+after(/deadlines/expiry.datetime)> true | <+signed_by(/users/buyer.id)> true)
  }
}
```

Transition predicates use the same predicate names inside governing models:

```modality
pending -> executed [+threshold("2", /treasury/signers)]
```

## Custom WASM Predicates

WASM predicates are intended custom predicate modules. They are not part of the
current local first-contract validator evidence matrix unless the predicate is
explicitly listed above. The local validator now derives `post_to_path(/path)`
from the pending commit body directly and `has_property(/path, "a.b")` from
accepted-state JSON directly; other WASM-style predicate-test inputs remain
explicit JSON until a validator path documents their replay binding.

```bash
modal predicate create --name my_predicate --output ./predicates/
```

Then reference in contracts:

```modality
wasm(/predicates/my_predicate.wasm, arg1, arg2)
```
