---
sidebar_position: 5
title: Predicate Commands
---

# Predicate Commands (`modal predicate`)

Manage and test predicates — the cryptographic conditions that guard transitions.

## List Predicates

```bash
modal predicate list [OPTIONS]
```

List all available predicates.

**Options:**
| Option | Description |
|--------|-------------|
| `--builtin` | Show only built-in predicates |
| `--custom` | Show only custom predicates |
| `--verbose` | Show detailed descriptions |

**Example output:**
```
BUILTIN PREDICATES:
  signed_by        Verify ed25519 signature
  threshold        n-of-m multisig verification
  oracle_attests   External oracle attestation
  before           Timestamp before constraint
  after            Timestamp after constraint
  hash_matches     SHA256 preimage verification
  num_eq           Numeric equality
  num_lt           Numeric less than
  num_gt           Numeric greater than
  num_lte          Numeric less than or equal
  num_gte          Numeric greater than or equal
  text_eq          Text equality
  text_contains    Text contains substring
  bool_is          Boolean value check
```

## Predicate Info

```bash
modal predicate info <NAME>
```

Get detailed information about a predicate.

**Example:**
```bash
modal predicate info signed_by
```

**Output:**
```
PREDICATE: signed_by

DESCRIPTION:
  Verifies an ed25519 signature against a public key stored at a path.

PARAMETERS:
  path      Path to .id file containing public key
  
IMPLICIT:
  signature  Taken from commit signature
  message    The canonical commit hash being signed

USAGE IN MODALITY:
  +signed_by(/parties/alice.id)

EXAMPLE:
  transition APPROVE: pending -> approved
    +signed_by(/parties/alice.id)
```

## Test Predicate

```bash
modal predicate test <NAME> [OPTIONS]
```

Test a predicate with sample data.

**Options:**
| Option | Description |
|--------|-------------|
| `--data <JSON>` | Test data as JSON |
| `--file <FILE>` | Load test data from file |
| `--verbose` | Show detailed evaluation |

**Examples:**

```bash
# Test signed_by predicate
modal predicate test signed_by --data '{
  "path": "/parties/alice.id",
  "public_key": "ed25519:abc123...",
  "signature": "def456...",
  "message": "commit_hash_here"
}'

# Test threshold predicate
modal predicate test threshold --data '{
  "path": "/treasury/signers",
  "n": 2,
  "m": 3,
  "signatures": ["sig1", "sig2"]
}'

# Test from file
modal predicate test oracle_attests --file test-oracle.json
```

## Create Custom Predicate

```bash
modal predicate create <NAME> [OPTIONS]
```

Scaffold a new custom predicate project.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | Output directory |
| `--lang <LANG>` | Language (rust/assemblyscript) |

**Example:**
```bash
modal predicate create kyc_verified --path ./predicates/kyc
```

Creates:
```
predicates/kyc/
├── Cargo.toml
├── src/
│   └── lib.rs        # Predicate implementation
├── tests/
│   └── test.rs       # Test cases
└── README.md
```

## Standard Predicates Reference

### Signature Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `signed_by(path)` | Single signature | `+signed_by(/alice.id)` |
| `threshold(path, n, m)` | n-of-m multisig | `+threshold(/signers, 2, 3)` |

### Oracle Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `oracle_attests(oracle, claim, value)` | Oracle attestation | `+oracle_attests(/oracle.id, "price", ">100")` |

### Timestamp Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `before(path)` | Before timestamp | `+before(/deadline.datetime)` |
| `after(path)` | After timestamp | `+after(/unlock.datetime)` |

### Value Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `num_eq(path, value)` | Numeric equals | `+num_eq(/amount, 100)` |
| `num_gt(path, value)` | Numeric greater than | `+num_gt(/balance, 0)` |
| `num_lt(path, value)` | Numeric less than | `+num_lt(/count, 10)` |
| `num_gte(path, value)` | Greater or equal | `+num_gte(/price, 50)` |
| `num_lte(path, value)` | Less or equal | `+num_lte(/fee, 5)` |
| `text_eq(path, value)` | Text equals | `+text_eq(/status.text, "active")` |
| `text_contains(path, sub)` | Contains substring | `+text_contains(/name, "Inc")` |
| `bool_is(path, value)` | Boolean check | `+bool_is(/active.bool, true)` |

### Cryptographic Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `hash_matches(hash, preimage)` | SHA256 verification | `+hash_matches(/commit.hash, /reveal.text)` |
