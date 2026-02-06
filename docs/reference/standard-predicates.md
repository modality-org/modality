---
sidebar_position: 1
title: Standard Predicates
---

# Standard Predicates

Predicates are the building blocks for contract rules. They evaluate based on commit data and contract state.

## Path Predicates

### modifies

Checks if the commit writes to paths under a given prefix.

```modality
+modifies(/members)
```

**Arguments:**
- `path` — Path prefix to check

**Behavior:**
- Returns true if any path in the commit body starts with the given prefix
- Used for path-based access control rules

**Example:**
```modality
// Only allow membership changes if all members sign
always (+modifies(/members) implies +all_signed(/members))
```

## Signature Predicates

### signed_by

Verifies the commit is signed by a specific ed25519 key.

```modality
signed_by(/users/alice.id)
```

**Arguments:**
- `path` — Path to the public key in contract state

### any_signed

Verifies at least one member from a path has signed.

```modality
+any_signed(/members)
```

**Arguments:**
- `path` — Path prefix containing member public keys

**Behavior:**
- Enumerates all `.id` files under the path
- Passes if ANY member has a valid signature
- Used for "any member can act" patterns

### all_signed

Verifies ALL members from a path have signed.

```modality
+all_signed(/members)
```

**Arguments:**
- `path` — Path prefix containing member public keys

**Behavior:**
- Enumerates all `.id` files under the path  
- Passes only if EVERY member has a valid signature
- Used for "unanimous consent" patterns like adding members

### threshold

n-of-m multisig verification.

```modality
threshold(2, /treasury/signers)
```

**Arguments:**
- `n` — Minimum signatures required
- `signers_path` — Path to JSON array of authorized signer paths

**Features:**
- Prevents same signer from signing twice
- Rejects unauthorized signers
- Works with any n-of-m configuration

## Time Predicates

### before

Checks current time is before a deadline.

```modality
before(/deadlines/expiry.datetime)
```

### after

Checks current time is after a timestamp.

```modality
after(/deadlines/start.datetime)
```

## State Predicates

### bool_true / bool_false

Check boolean state values.

```modality
bool_true(/status/delivered.bool)
bool_false(/flags/cancelled.bool)
```

### text_eq

Compare text values.

```modality
text_eq(/status.text, "approved")
```

### num_eq / num_gt / num_gte / num_lt / num_lte

Numeric comparisons.

```modality
num_gte(/balance.num, 100)
num_lt(/deposit.num, /limit.num)
```

## Oracle Predicates

### oracle_attests

Verifies a signed attestation from a trusted oracle.

```modality
oracle_attests(/oracles/delivery.id, "delivered", "true")
```

**Arguments:**
- `oracle_path` — Path to oracle's public key
- `claim` — The claim type being attested
- `value` — Expected value (optional)

**Security features:**
- Verifies oracle signature
- Enforces attestation freshness
- Binds attestation to specific contract
- Prevents replay attacks

## Hash Predicates

### hash_matches

Verifies SHA256 hash commitment.

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
    signed_by(/users/alice.id) | signed_by(/users/bob.id)
  }
}

export default rule {
  starting_at $PARENT
  formula {
    // After deadline, only buyer can commit
    after(/deadlines/expiry.datetime) -> signed_by(/users/buyer.id)
  }
}

export default rule {
  starting_at $PARENT
  formula {
    // 2-of-3 multisig required
    threshold(2, /treasury/signers)
  }
}
```

## Custom WASM Predicates

You can create custom predicates as WASM modules:

```bash
modal predicate create --name my_predicate --output ./predicates/
```

Then reference in contracts:

```modality
wasm(/predicates/my_predicate.wasm, arg1, arg2)
```
