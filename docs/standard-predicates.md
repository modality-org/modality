# Standard Predicates in Network Genesis

## Overview

The network genesis contract includes standard predicates available to all contracts at `/_code/modal/*.wasm`.

## Available Predicates

### 1. signed_by
Verifies cryptographic signatures on data.

**Usage**:
```javascript
const result = await executor.evaluate_predicate(
  contractId,
  "/_code/modal/signed_by.wasm",
  {
    message: "data to verify",
    signature: "base64_signature",
    public_key: "base64_public_key"
  },
  context
);
```

### 2. amount_in_range
Checks if a numeric amount is within bounds.

**Usage**:
```javascript
const result = await executor.evaluate_predicate(
  contractId,
  "/_code/modal/amount_in_range.wasm",
  {
    amount: 100,
    min: 0,
    max: 1000
  },
  context
);
```

### 3. has_property
Checks if a JSON object has a property (supports nested paths).

**Usage**:
```javascript
const result = await executor.evaluate_predicate(
  contractId,
  "/_code/modal/has_property.wasm",
  {
    object: { user: { address: { city: "NYC" } } },
    property_path: "user.address.city"
  },
  context
);
```

### 4. timestamp_valid
Validates timestamp constraints.

**Usage**:
```javascript
const result = await executor.evaluate_predicate(
  contractId,
  "/_code/modal/timestamp_valid.wasm",
  {
    timestamp: 1234567890,
    max_age_seconds: 3600,
    min_age_seconds: 0
  },
  context
);
```

### 5. post_to_path
Checks if a commit includes a POST action to a path.

**Usage**:
```javascript
const result = await executor.evaluate_predicate(
  contractId,
  "/_code/modal/post_to_path.wasm",
  {
    commit: {
      actions: [
        { method: "post", path: "/config/value" }
      ]
    },
    path: "/config/value",
    exact_match: true
  },
  context
);
```

## In Modal Formulas

Predicates evaluate to propositions:

```
# Simple predicate check
+amount_in_range({"amount": 100, "min": 0, "max": 1000})

# Used in formulas
formula valid_transfer:
  <+amount_in_range(...)> <+signed_by(...)> true
```

## Result Format

All predicates return:
```json
{
  "valid": true,        // Boolean result → becomes +predicate or -predicate
  "gas_used": 250,      // Gas consumed
  "errors": []          // Error messages if validation failed
}
```

## Building Predicates

To compile the standard predicates:

```bash
cd rust/modal-wasm-validation
./build-predicates.sh
```

This will generate WASM files in `build/wasm/predicates/` which are automatically included in new genesis contracts.

## Custom Predicates

Contracts can create their own predicates:

```bash
modal contract wasm-upload \
  --dir ./my-contract \
  --wasm-file ./my_predicate.wasm \
  --module-name "/custom/my_predicate"
```

Then reference it as:
- From same contract: `/_code/custom/my_predicate.wasm`
- From other contracts: `@{contract_id}/_code/custom/my_predicate.wasm`

