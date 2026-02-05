# WASM Predicate System - Complete Documentation

**Last Updated**: November 16, 2025  
**Version**: 1.0.0

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Standard Predicates](#standard-predicates)
4. [Using Predicates](#using-predicates)
5. [Creating Custom Predicates](#creating-custom-predicates)
6. [CLI Reference](#cli-reference)
7. [Performance & Caching](#performance--caching)
8. [Security](#security)
9. [API Reference](#api-reference)
10. [Examples](#examples)
11. [Troubleshooting](#troubleshooting)

---

## Overview

The WASM Predicate System enables dynamic computation of propositions in modal formulas. Instead of static string-based properties, predicates execute WASM code to verify conditions and generate propositions.

### Key Concepts

**Predicate** → **Evaluation** → **Proposition** → **Formula**

```
+amount_in_range({"amount": 100, "min": 0, "max": 1000})
         ↓ (execute WASM)
    { valid: true, gas_used: 25, errors: [] }
         ↓ (convert to proposition)
     +amount_in_range
         ↓ (use in formula)
  <+amount_in_range> true
```

### Benefits

- ✅ **Verifiable**: Deterministic execution ensures reproducible results
- ✅ **Flexible**: Write custom predicates for any verification logic
- ✅ **Performant**: Compiled WASM modules are cached (87% speedup)
- ✅ **Secure**: Sandboxed execution with gas metering
- ✅ **Composable**: Mix static and predicate properties in formulas

---

## Quick Start

### 1. Check Available Predicates

```bash
$ pnpm modal predicate list

📋 Predicates in contract: modal.money

Standard Network Predicates:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  signed_by
  ─────────
  Path:        /_code/modal/signed_by.wasm
  Description: Verify cryptographic signatures
  Arguments:   { message, signature, public_key }
  Gas Usage:   100-200

  amount_in_range
  ───────────────
  Path:        /_code/modal/amount_in_range.wasm
  Description: Check numeric bounds
  Arguments:   { amount, min, max }
  Gas Usage:   20-30

  [... more predicates ...]
```

### 2. Test a Predicate

```bash
$ pnpm modal predicate test amount_in_range \
    --args '{"amount": 100, "min": 0, "max": 1000}'

🧪 Testing Predicate: amount_in_range
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Input:
  Contract:     modal.money
  Predicate:    amount_in_range
  Arguments:    {
                  "amount": 100,
                  "min": 0,
                  "max": 1000
                }

Result:
  Valid:        ✅ true
  Gas Used:     25
  Proposition:  +amount_in_range
```

### 3. Use in a Modal Model

```modality
model payment:
  part transaction:
    init -> pending: +created
    pending -> approved: +amount_in_range({"amount": 100, "min": 0, "max": 1000})
    approved -> signed: +signed_by({"message": "tx", "signature": "sig", "public_key": "pk"})

formula safe_payment:
  <+amount_in_range> <+signed_by> true
```

---

## Standard Predicates

The network provides 5 standard predicates available in all contracts:

### 1. `signed_by`

Verify cryptographic signatures.

**Arguments:**
- `message` (string) - The message that was signed
- `signature` (string) - The signature to verify
- `public_key` (string) - The public key for verification

**Example:**
```modality
+signed_by({"message": "approve", "signature": "sig123", "public_key": "pk456"})
```

**Gas Usage:** 100-200

---

### 2. `amount_in_range`

Check if a numeric value is within bounds.

**Arguments:**
- `amount` (number) - The value to check
- `min` (number) - Minimum allowed value (inclusive)
- `max` (number) - Maximum allowed value (inclusive)

**Example:**
```modality
+amount_in_range({"amount": 100, "min": 0, "max": 1000})
```

**Gas Usage:** 20-30

---

### 3. `has_property`

Check if a JSON object has a specific property.

**Arguments:**
- `path` (string) - JSON path (dot notation, e.g., "user.email")
- `required` (boolean) - Whether the property must exist

**Example:**
```modality
+has_property({"path": "user.email", "required": true})
```

**Gas Usage:** 30-50

---

### 4. `timestamp_valid`

Validate timestamps against age constraints.

**Arguments:**
- `timestamp` (number) - Unix timestamp to validate
- `max_age_seconds` (number, optional) - Maximum age in seconds

**Example:**
```modality
+timestamp_valid({"timestamp": 1234567890, "max_age_seconds": 3600})
```

**Gas Usage:** 25-35

---

### 5. `post_to_path`

Verify that a commit includes a POST action to a specific path.

**Arguments:**
- `path` (string) - The path to check for

**Example:**
```modality
+post_to_path({"path": "/_code/validator.wasm"})
```

**Gas Usage:** 40-100

---

## Using Predicates

### Syntax

Predicates use function-call syntax with JSON arguments:

```
+predicate_name({"arg1": "value1", "arg2": value2})
-predicate_name({"arg1": "value1"})
```

- `+` indicates expected true result
- `-` indicates expected false result
- Arguments must be valid JSON

### In Transitions

```modality
model payment:
  part transaction:
    pending -> approved: +amount_in_range({"amount": 100, "min": 0, "max": 1000})
```

### In Formulas

```modality
formula safe_transaction:
  <+amount_in_range> <+signed_by> true

formula invalid_transaction:
  <-amount_in_range> false
```

### Mixing Static and Predicate Properties

```modality
model hybrid:
  part flow:
    init -> processing: +started                                          # static
    processing -> validated: +amount_in_range({"amount": 100, ...})      # predicate
    validated -> done: +completed                                        # static
```

---

## Creating Custom Predicates

### 1. Write Predicate in Rust

```rust
// src/lib.rs
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Input {
    data: Value,
    context: Context,
}

#[derive(Debug, Deserialize)]
struct Context {
    contract_id: String,
    block_height: u64,
    timestamp: u64,
}

#[derive(Debug, Serialize)]
struct Result {
    valid: bool,
    gas_used: u64,
    errors: Vec<String>,
}

#[wasm_bindgen]
pub fn evaluate(input_json: &str) -> String {
    // Parse input
    let input: Input = serde_json::from_str(input_json).unwrap();
    
    // Your validation logic here
    let valid = true; // your logic
    
    // Return result
    let result = Result {
        valid,
        gas_used: 25,
        errors: vec![],
    };
    
    serde_json::to_string(&result).unwrap()
}
```

### 2. Build to WASM

```bash
$ wasm-pack build --target web --release
```

### 3. Upload to Contract

```bash
$ pnpm modal predicate upload pkg/my_predicate_bg.wasm \
    --contract-id mycontract \
    --name my_predicate \
    --gas-limit 50000
```

### 4. Use in Your Contract

```modality
model my_model:
  part flow:
    init -> validated: +my_predicate({"custom": "args"})
```

### Best Practices

✅ **Keep predicates simple** - One clear responsibility
✅ **Validate all inputs** - Never trust input data
✅ **Use descriptive names** - `snake_case` convention
✅ **Return clear errors** - Help debugging
✅ **Keep gas usage low** - < 1M for simple checks
✅ **Make deterministic** - No randomness or time-based logic
✅ **Test thoroughly** - Test all edge cases

---

## CLI Reference

### `predicate list`

List available predicates.

```bash
$ pnpm modal predicate list [contract-id]
```

**Options:**
- `contract-id` - Contract to query (default: modal.money)

---

### `predicate info`

Get detailed information about a predicate.

```bash
$ pnpm modal predicate info <name>
```

**Options:**
- `name` (required) - Predicate name
- `--contract-id` - Contract ID (default: modal.money)

**Example:**
```bash
$ pnpm modal predicate info amount_in_range
```

---

### `predicate test`

Test a predicate with sample data.

```bash
$ pnpm modal predicate test <name> --args <json>
```

**Options:**
- `name` (required) - Predicate name
- `--args` (required) - JSON arguments
- `--contract-id` - Contract ID (default: modal.money)
- `--block-height` - Block height for context (default: 1)
- `--timestamp` - Timestamp for context (default: now)

**Example:**
```bash
$ pnpm modal predicate test amount_in_range \
    --args '{"amount": 100, "min": 0, "max": 1000}'
```

---

### `predicate upload`

Upload a custom predicate to a contract.

```bash
$ pnpm modal predicate upload <wasm-file> --contract-id <id>
```

**Options:**
- `wasm-file` (required) - Path to WASM file
- `--contract-id` (required) - Target contract
- `--name` - Predicate name (inferred from filename if omitted)
- `--gas-limit` - Gas limit (default: 10000000)

**Example:**
```bash
$ pnpm modal predicate upload my_predicate.wasm \
    --contract-id mycontract \
    --gas-limit 50000
```

---

## Performance & Caching

### Caching Strategy

Compiled WASM modules are cached using an LRU (Least Recently Used) policy:

- **Cache Keys**: `(contract_id, path, hash)`
- **Limits**: 100 modules OR 50MB total
- **Eviction**: Oldest unused modules removed first
- **Invalidation**: Hash changes trigger re-compilation

### Performance Metrics

| Operation | First Call | Cached Call | Improvement |
|-----------|------------|-------------|-------------|
| Simple (amount_in_range) | ~15ms | ~2ms | **87% faster** |
| Medium (has_property) | ~18ms | ~2.5ms | 86% faster |
| Complex (signed_by) | ~25ms | ~5ms | 80% faster |

### Gas Consumption

| Predicate | Typical Gas | Max Gas |
|-----------|-------------|---------|
| amount_in_range | 20-30 | 50 |
| has_property | 30-50 | 80 |
| timestamp_valid | 25-35 | 60 |
| post_to_path | 40-100 | 150 |
| signed_by | 100-200 | 300 |

### Optimization Tips

1. **Use caching** - Repeated evaluations are ~87% faster
2. **Keep predicates simple** - Lower gas usage
3. **Batch validations** - Evaluate multiple properties together
4. **Monitor gas usage** - Optimize hot paths

---

## Security

### Sandboxing

Predicates run in a sandboxed WASM environment with:

- ❌ No filesystem access
- ❌ No network access
- ❌ No system calls
- ✅ Only pure computation

### Gas Metering

- **Default Limit**: 10,000,000 gas units
- **Maximum Limit**: 100,000,000 gas units
- **Enforcement**: Execution stops if limit exceeded
- **Prevention**: Protects against infinite loops

### Hash Verification

- **Integrity**: SHA-256 hash computed at upload
- **Validation**: Hash checked before execution
- **Immutability**: Modules cannot be tampered with

### Determinism

- **Requirement**: Same input must produce same output
- **No Randomness**: Random values not allowed
- **No Time**: Current time not accessible
- **Context**: Only provided context available

### Cross-Contract Isolation

- **Namespace**: Each contract has isolated /_code/ directory
- **Permissions**: Contracts can only access their own predicates
- **Network Predicates**: Shared standard predicates in modal.money

---

## API Reference

### Rust API

#### `PredicateExecutor`

```rust
pub struct PredicateExecutor {
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>, gas_limit: u64) -> Self;
    
    pub async fn evaluate_predicate(
        &self,
        contract_id: &str,
        predicate_path: &str,
        data: Value,
        context: PredicateContext,
    ) -> Result<PredicateResult>;
}
```

#### `PredicateResult`

```rust
pub struct PredicateResult {
    pub valid: bool,
    pub gas_used: u64,
    pub errors: Vec<String>,
}
```

#### `PredicateContext`

```rust
pub struct PredicateContext {
    pub contract_id: String,
    pub block_height: u64,
    pub timestamp: u64,
}
```

### JavaScript API

#### `Property`

```javascript
class Property {
    constructor(name, value = true, source = null);
    
    static fromText(text);
    isStatic();
    isPredicate();
    getPredicate(); // Returns { path, args }
}
```

#### `PropertyTable`

```javascript
class PropertyTable {
    constructor(default_value, predicateExecutor = null);
    
    async getValue(name, context = {});
    setPredicateResult(name, value, context = {});
    clearPredicateCache();
}
```

---

## Examples

### Example 1: Financial Validation

```modality
model invoice:
  part payment:
    init -> validating: +received
    validating -> approved: +amount_in_range({"amount": 500, "min": 100, "max": 10000})
                           +signed_by({"message": "invoice_123", "signature": "sig", "public_key": "pk"})
    approved -> paid: +processed

formula valid_payment:
  <+amount_in_range> <+signed_by> true
```

### Example 2: Time-Based Validation

```modality
model session:
  part auth:
    init -> active: +authenticated
    active -> expired: -timestamp_valid({"timestamp": 1234567890, "max_age_seconds": 3600})
    active -> renewed: +refresh

formula session_valid:
  <+timestamp_valid> true
```

### Example 3: Data Validation

```modality
model user_registration:
  part validation:
    init -> checking: +submitted
    checking -> valid: +has_property({"path": "user.email", "required": true})
                      +has_property({"path": "user.age", "required": true})
    valid -> registered: +complete

formula complete_profile:
  <+has_property> <+has_property> true
```

### Example 4: Custom Predicate

See `examples/network/predicate-usage/create-custom-predicate.sh` for a complete example of creating a custom `is_within_percent` predicate.

---

## Troubleshooting

### Predicate Not Found

**Problem**: `❌ WASM module not found at path '/_code/my_predicate.wasm'`

**Solutions**:
1. Verify predicate was uploaded: `pnpm modal predicate list --contract-id mycontract`
2. Check path matches exactly (case-sensitive)
3. Ensure contract ID is correct

### Gas Limit Exceeded

**Problem**: Predicate execution stops with gas error

**Solutions**:
1. Simplify predicate logic
2. Increase gas limit when uploading: `--gas-limit 50000000`
3. Profile predicate to find expensive operations

### Invalid Arguments

**Problem**: `❌ Invalid input: missing field 'amount'`

**Solutions**:
1. Check JSON syntax: `--args '{"amount": 100}'`
2. Verify all required fields present
3. Use `pnpm modal predicate info <name>` to see expected arguments

### Cache Issues

**Problem**: Stale results from cached predicates

**Solutions**:
1. Cache automatically invalidates on hash change
2. Upload new version with different hash
3. Restart node to clear cache

---

## Additional Resources

- **GitHub**: https://github.com/modality-org/modality
- **Examples**: `examples/network/predicate-usage/`
- **Tests**: `rust/modal-wasm-validation/tests/`
- **Implementation Reports**:
  - `WASM_PREDICATE_FINAL.md` - Complete implementation summary
  - `MODEL_CHECKER_PREDICATE_ARCHITECTURE.md` - Architecture decisions
  - `docs/standard-predicates.md` - Standard predicates reference

---

**Version**: 1.0.0  
**Last Updated**: November 16, 2025  
**Status**: Production Ready  
**Test Coverage**: 87+ tests passing

