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

List predicates for a contract. By default this uses the built-in
`modal.money` predicate catalog.

**Options:**
| Option | Description |
|--------|-------------|
| `--contract-id <CONTRACT_ID>` | Contract ID to list predicates from (default: `modal.money`) |

**Example output:**
```
BUILTIN PREDICATES:
  signed_by         Verify cryptographic signatures
  amount_in_range   Check numeric bounds
  has_property      Check JSON property existence
  timestamp_valid   Validate timestamp constraints
  post_to_path      Verify commit actions
```

## Predicate Info

```bash
modal predicate info <NAME> [OPTIONS]
```

Get detailed information about a predicate.

**Options:**
| Option | Description |
|--------|-------------|
| `--contract-id <CONTRACT_ID>` | Contract ID to read predicate metadata from (default: `modal.money`) |

**Example:**
```bash
modal predicate info signed_by
```

**Output:**
```
PREDICATE: signed_by

DESCRIPTION:
  Verify cryptographic signatures using public key cryptography

PARAMETERS:
  message      The message that was signed
  signature    The signature to verify
  public_key   The public key to verify against

USAGE IN MODALITY:
  +signed_by({"message": "hello", "signature": "sig123", "public_key": "pk456"})

EXAMPLE:
  modal predicate test signed_by --args '{"message":"hello","signature":"sig123","public_key":"pk456"}'
```

## Test Predicate

```bash
modal predicate test <NAME> [OPTIONS]
```

Test a predicate with sample data. This command prints a simulated result from
the predicate helper logic; actual predicate execution still requires a running
node.

**Options:**
| Option | Description |
|--------|-------------|
| `--args <ARGS>` | Arguments as a JSON string |
| `--contract-id <CONTRACT_ID>` | Contract ID for context (default: `modal.money`) |
| `--block-height <BLOCK_HEIGHT>` | Block height for context (default: `1`) |
| `--timestamp <TIMESTAMP>` | Unix timestamp for context |

**Examples:**

```bash
# Test signed_by predicate
modal predicate test signed_by --args '{
  "message": "commit_hash_here",
  "signature": "def456...",
  "public_key": "ed25519:abc123..."
}'

# Test amount_in_range predicate
modal predicate test amount_in_range --args '{
  "amount": 100,
  "min": 0,
  "max": 1000
}'

# Test with explicit context
modal predicate test post_to_path \
  --contract-id modal.money \
  --block-height 42 \
  --timestamp 1700000000 \
  --args '{"path":"/_code/validator.wasm"}'
```

## Create Custom Predicate

```bash
modal predicate create [OPTIONS]
```

Scaffold a new custom predicate project.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Directory to create the predicate project in |
| `--name <NAME>` | Predicate name (defaults to directory name) |

**Example:**
```bash
modal predicate create --dir ./predicates/kyc --name kyc_verified
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
| `signed_by` | Simulated signature argument check in `modal predicate test`; local first-contract enforcement is documented in the standard predicate reference. | `+signed_by({"message":"hello","signature":"sig123","public_key":"pk456"})` |

### Value Predicates

| Predicate | Description | Usage |
|-----------|-------------|-------|
| `amount_in_range` | Check numeric bounds in simulated predicate testing. | `+amount_in_range({"amount":100,"min":0,"max":1000})` |
| `has_property` | Check JSON property existence in simulated predicate testing. | `+has_property({"path":"user.email","required":true})` |
| `timestamp_valid` | Validate timestamp arguments in simulated predicate testing. | `+timestamp_valid({"timestamp":1234567890,"max_age_seconds":3600})` |
| `post_to_path` | Check for a path argument in simulated predicate testing. | `+post_to_path({"path":"/_code/validator.wasm"})` |

For implementation-backed first-contract predicates such as
`signed_by(/path.id)`, `threshold("2", /path)`, `any_signed(/path)`,
`all_signed(/path)`, and `modifies(/path)`, see the standard predicate
reference.
