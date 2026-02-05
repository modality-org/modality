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
