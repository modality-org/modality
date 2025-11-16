# Standard Predicates

This directory contains the standard predicates available in the modal money network genesis contract at `/_code/modal/*.wasm`.

## Available Predicates

### 1. signed_by
**Path**: `/_code/modal/signed_by.wasm`

Verifies cryptographic signatures on data.

**Input**:
```json
{
  "data": {
    "message": "data to verify",
    "signature": "base64_signature",
    "public_key": "base64_public_key"
  },
  "context": {
    "contract_id": "...",
    "block_height": 1234,
    "timestamp": 1234567890
  }
}
```

**Output**: `PredicateResult` with `valid: true/false`

**Note**: Currently a placeholder. Full signature verification implementation pending.

### 2. amount_in_range
**Path**: `/_code/modal/amount_in_range.wasm`

Checks if a numeric amount is within a specified range.

**Input**:
```json
{
  "data": {
    "amount": 100,
    "min": 0,
    "max": 1000
  },
  "context": {...}
}
```

**Output**: `valid: true` if `min <= amount <= max`

### 3. has_property
**Path**: `/_code/modal/has_property.wasm`

Checks if a JSON object has a specific property. Supports dot notation for nested properties.

**Input**:
```json
{
  "data": {
    "object": {"user": {"address": {"city": "NYC"}}},
    "property_path": "user.address.city"
  },
  "context": {...}
}
```

**Output**: `valid: true` if property exists at path

### 4. timestamp_valid
**Path**: `/_code/modal/timestamp_valid.wasm`

Validates that a timestamp is within acceptable bounds relative to the current time.

**Input**:
```json
{
  "data": {
    "timestamp": 1234567890,
    "max_age_seconds": 3600,
    "min_age_seconds": 0
  },
  "context": {...}
}
```

**Output**: `valid: true` if timestamp is within age constraints

### 5. post_to_path
**Path**: `/_code/modal/post_to_path.wasm`

Checks if a commit includes a POST action to a specific path.

**Input**:
```json
{
  "data": {
    "commit": {
      "actions": [
        {"method": "post", "path": "/config/value"},
        {"method": "send", ...}
      ]
    },
    "path": "/config/value",
    "exact_match": true
  },
  "context": {...}
}
```

**Output**: `valid: true` if matching POST action found

## Using Predicates

### From Contracts
Predicates can be called from within modal formulas and property evaluations:

```
# Property that checks if amount is in range
+amount_in_range({"amount": 100, "min": 0, "max": 1000})

# Can be used in formulas
formula valid_transfer:
  <+amount_in_range(...)> <+signed_by(...)> true
```

### Cross-Contract References
Contracts can reference predicates from other contracts:

```
# Reference the network genesis contract (default)
/_code/modal/signed_by.wasm

# Reference a custom predicate from another contract
@{contract_id}/_code/custom_validator.wasm
```

## Creating Custom Predicates

Contracts can create their own predicates by posting WASM modules to `/_code/` paths:

```bash
modal contract wasm-upload \
  --dir ./my-contract \
  --wasm-file ./my_predicate.wasm \
  --module-name "/custom/my_predicate" \
  --gas-limit 5000000
```

### Predicate Interface

All predicates must:
1. Export a function called `evaluate`
2. Take JSON input with `data` and `context` fields
3. Return JSON with `valid`, `gas_used`, and `errors` fields

Example in Rust:
```rust
use modal_wasm_validation::{PredicateInput, PredicateResult};

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    // Your validation logic here
    let is_valid = /* ... */;
    
    if is_valid {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Validation failed".to_string()])
    }
}
```

## Gas Metering

All predicates execute with gas metering to prevent infinite loops:
- Default gas limit: 10,000,000 instructions
- Maximum gas limit: 100,000,000 instructions
- Custom limits can be specified when uploading

## Caching

Compiled WASM modules are cached for performance:
- LRU eviction when cache is full
- Network contract predicates prioritized
- Cache hit rates typically >80%

## Security

- Sandboxed execution (no filesystem, network access)
- Hash verification prevents tampering
- Deterministic execution required
- Cross-contract execution limits prevent recursion

