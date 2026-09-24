# Standard Predicates

This directory contains standard predicate modules intended for the modal money
network genesis contract at `/_code/modal/*.wasm`.

These modules are locally unit-tested predicate evaluators. They are not, by
themselves, evidence that the local first-contract validator can derive each
predicate fact from replayed contract commits. For the current local
first-contract evidence boundary, use `docs/reference/standard-predicates.md`.

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
The checked amount is explicit predicate-test input for this WASM module. The
`modality-cli-contract` local model-governance path also derives
`amount_in_range(/path, "min", "max")` directly from accepted-state numbers.
Bounds can be quoted numeric values or paths to accepted-state numeric values.

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
The checked object is explicit predicate-test input for this WASM module. The
`modality-cli-contract` local model-governance path also derives
`has_property(/path, "a.b")` directly from accepted-state JSON, following
dot-separated object keys on previously committed state.
It also derives `state_exists(/path)` from accepted-state path existence.
It also derives `text_eq` from accepted-state strings when comparing a state
path to a literal string or to another accepted-state path.
It also derives `text_contains` from accepted-state strings when checking for a
literal substring.
It also derives `text_starts_with` and `text_ends_with` from accepted-state
strings when checking literal prefixes or suffixes.
It also derives `amount_in_range` from accepted-state numbers when comparing a
state path to inclusive literal or accepted-state numeric bounds.
It also derives `num_eq`, `num_gt`, `num_gte`, `num_lt`, and `num_lte` from
accepted-state numbers when comparing a state path to a literal number or to
another accepted-state number path.
It also derives `bool_true` and `bool_false` from accepted-state booleans.

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
The "current time" is `context.timestamp`; a contract-log validator must
document the trusted clock source before this can support local deadline
evidence.

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
The `modality-cli-contract` local model-governance path also derives
`post_to_path(/path)` directly from the pending commit body, matching `POST`
actions to the path itself or descendants.

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

### Oracle attestation replay boundary

The `oracle_attests` evaluator is unit-tested extension code, not current local
first-contract replay evidence. Its test input now requires the attestation's
signed payload to bind the oracle key, oracle path, claim, value, contract id,
pending commit hash, and timestamp. When `replay_bundle_json` is supplied, the
evaluator parses the canonical `oracle_attests` replay-bundle envelope and
requires the bundle's positive `max_age_seconds` freshness policy to match the
predicate input before the bundle can be accepted. Replay-bundle inputs must
also include `accepted_state_oracle_keys`, the replayed accepted-state oracle-key
map keyed by path; the evaluator derives the key at `expected_oracle_path`,
requires that looked-up key to be valid hex-encoded ed25519 public-key material,
checks that any scalar `expected_oracle_pubkey` agrees with the map lookup, and
requires the bundle attestation to match that looked-up key. That evaluator
boundary is paired with `modality-common` replay helpers that derive
`accepted_state_oracle_keys` from the actual accepted contract state by reading
string values posted at `/oracles/**/*.id` paths, after updates and deletes are
applied, before a pending commit is expanded or checked. The CLI and validator
WASM program context now carries that derived map as
`context.accepted_state_oracle_keys`, so replayed invoke programs can build
oracle replay bundles from the same accepted-state key material the verifier
will check. The validator predicate executor now also has an explicit
replay-evidence handoff that injects those replay-derived oracle keys into an
`oracle_attests` predicate input only when that input already carries
`replay_bundle_json`, and it preserves any explicit predicate-supplied key map
instead of overwriting it. This does not yet make `oracle_attests`
first-contract-local validator evidence; local and hub replay still report it
as missing external evidence until the contract-log replay path supplies replay
bundles end to end. The contract processor now has a replay-state-aware
predicate evaluation entry point that derives the parent commit's accepted-state
oracle-key map from the sequenced parent chain and routes replay-bundle
predicate input through that handoff. The evaluator rejects missing
replay-bundle freshness policies, bundle/input freshness
mismatches, missing accepted-state oracle-key lookup maps or path entries,
malformed accepted-state oracle keys, scalar/key-map mismatches, accepted-state
oracle key mismatches, malformed JSON, non-canonical JSON bytes, wrong predicate
names, and attestations that differ from the predicate input before an oracle
claim can pass. The evaluator also rejects missing oracle-path bindings,
mismatched oracle paths, missing pending-commit bindings, and mismatched pending
commit hashes. A contract-log validator still needs to pass the replay bundle
to the oracle predicate evaluator before this can be reported as checked
validator evidence.

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
use modality_wasm_validation::{PredicateInput, PredicateResult};

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
