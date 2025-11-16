# WASM Predicates Example

This example demonstrates using WASM predicates for modal proposition evaluation.

## Overview

Predicates are WASM functions that evaluate to boolean propositions. They can be:
- **Standard predicates**: Available network-wide at `/_code/modal/*.wasm`
- **Custom predicates**: Created by contracts at `/_code/*.wasm`

## Setup

```bash
# Start from examples/network
cd examples/network/predicate-usage

# Initialize test environment
./00-setup.sh

# Create a test node
./01-create-node.sh
```

## Example 1: Using Standard Predicates

```javascript
const { PredicateExecutor } = require('@modality-dev/sdk');

// Create executor
const executor = new PredicateExecutor(datastore, 10_000_000);

// Evaluate amount_in_range predicate
const result = await executor.evaluate_predicate(
  "contract123",
  "/_code/modal/amount_in_range.wasm",
  {
    amount: 100,
    min: 0,
    max: 1000
  },
  {
    contract_id: "contract123",
    block_height: 1234,
    timestamp: Date.now() / 1000
  }
);

console.log(result.valid); // true
console.log(result.gas_used); // ~30
console.log(result.errors); // []

// Convert to proposition
const proposition = executor.result_to_proposition("amount_in_range", result);
console.log(proposition); // "+amount_in_range"
```

## Example 2: Checking Properties

```javascript
// Check if object has nested property
const hasProperty = await executor.evaluate_predicate(
  "contract123",
  "/_code/modal/has_property.wasm",
  {
    object: {
      user: {
        profile: {
          verified: true
        }
      }
    },
    property_path: "user.profile.verified"
  },
  context
);

console.log(hasProperty.valid); // true
```

## Example 3: Validating Timestamps

```javascript
// Check if timestamp is not too old
const timestampValid = await executor.evaluate_predicate(
  "contract123",
  "/_code/modal/timestamp_valid.wasm",
  {
    timestamp: Math.floor(Date.now() / 1000) - 1800, // 30 minutes ago
    max_age_seconds: 3600 // Max 1 hour old
  },
  context
);

console.log(timestampValid.valid); // true
```

## Example 4: Verifying Commit Actions

```javascript
// Check if commit includes a POST to /config
const hasPost = await executor.evaluate_predicate(
  "contract123",
  "/_code/modal/post_to_path.wasm",
  {
    commit: {
      actions: [
        { method: "post", path: "/config/value" },
        { method: "send", to: "other_contract" }
      ]
    },
    path: "/config",
    exact_match: false // Prefix match
  },
  context
);

console.log(hasPost.valid); // true
```

## Example 5: Cache Benefits

```javascript
// First call - cache miss
const start1 = Date.now();
await executor.evaluate_predicate(contractId, predicatePath, data, context);
const time1 = Date.now() - start1;
console.log(`First call: ${time1}ms`); // e.g., 15ms (compilation)

// Second call - cache hit!
const start2 = Date.now();
await executor.evaluate_predicate(contractId, predicatePath, data, context);
const time2 = Date.now() - start2;
console.log(`Second call: ${time2}ms`); // e.g., 2ms (cached)

// Check cache stats
const stats = await executor.cache_stats();
console.log(`Hit rate: ${(stats.hit_rate * 100).toFixed(1)}%`);
console.log(`Cached modules: ${stats.entries}`);
```

## Example 6: Cross-Contract Predicates

```javascript
// Reference predicate from another contract
const otherContractId = "abc123...";
const result = await executor.evaluate_predicate(
  "current_contract",
  `@${otherContractId}/_code/custom_validator.wasm`,
  { /* data */ },
  context
);
```

## Using in Modal Formulas

```modality
model payment_system:
  part transaction:
    pending -> approved: +amount_in_range +signed_by
    pending -> rejected: -amount_in_range
    pending -> rejected: -signed_by
    approved -> completed: +timestamp_valid
    
formula safe_payment:
  <+amount_in_range> <+signed_by> <+timestamp_valid> true
```

## Gas Considerations

Different predicates have different gas costs:
- `amount_in_range`: ~20-30 gas (simple arithmetic)
- `has_property`: ~30-50 gas (depends on nesting depth)
- `timestamp_valid`: ~25-35 gas (simple comparisons)
- `post_to_path`: ~40-100 gas (depends on commit size)
- `signed_by`: ~100-200 gas (cryptographic operations)

## Testing

Run the tests:
```bash
./test.sh
```

Expected output:
```
✓ Standard predicates available in genesis
✓ amount_in_range evaluates correctly
✓ has_property checks nested paths
✓ timestamp_valid enforces constraints
✓ post_to_path finds actions
✓ Cache improves performance
```

## Notes

- Predicates are deterministic (same input → same output)
- All predicates have gas limits (default: 10M instructions)
- Compiled modules are cached for performance
- Hash verification ensures integrity
- Predicates execute in sandbox (no I/O)

