# Predicate Design for Synthesis

Thinking through predicates needed for practical contract synthesis.

## Current Predicates

### `signed_by(path)`
Verifies ed25519 signature from identity at path.
```modality
+signed_by(/users/alice.id)
```

**Implementation:** WASM module, uses ed25519-dalek

---

## Predicates Needed for Common Patterns

### 1. Time-Based: `before(timestamp)` / `after(timestamp)`

Many contracts need deadlines:
- "Bob must deliver within 7 days"
- "Alice can reclaim funds after deadline"

```modality
// Delivery must happen before deadline
always([+DELIVER] true -> <+before(/state/deadline.datetime)> true)

// Refund only after deadline
always([+REFUND] true -> <+after(/state/deadline.datetime)> true)
```

**Implementation consideration:** How to get current time trustlessly?
- Option A: Commit includes timestamp, verified against block time
- Option B: Oracle attests to time
- Option C: Chain height as proxy for time

### 2. Amount-Based: `amount_equals(value)` / `amount_gte(value)`

Payment verification:
- "Deposit must be exactly 100 tokens"
- "Payment must be at least the agreed price"

```modality
// Exact payment
always([+DEPOSIT] true -> <+amount_equals(/state/price.json)> true)

// Minimum payment
always([+PAY] true -> <+amount_gte(/state/minimum.json)> true)
```

**Implementation consideration:** What's the value source?
- Path references state file with amount
- Or inline literal: `amount_equals(100)`

### 3. Hash-Based: `hash_matches(commitment)` / `preimage_of(hash)`

Atomic swaps need hash-locked commitments:
- "Reveal must match the committed hash"
- "Only proceed if preimage is valid"

```modality
// Reveal phase
always([+REVEAL_A] true -> <+preimage_of(/state/commitment_a.hash)> true)

// Hash time-locked contracts
always([+CLAIM] true -> <+hash_matches(/state/secret.hash)> true)
```

**Implementation:** SHA256 or Blake3, committed value in state

### 4. Oracle-Based: `oracle_attests(condition)`

External verification:
- "Oracle confirms goods were delivered"
- "Reputation service approves seller"

```modality
// Oracle attestation
always([+RELEASE] true -> <+oracle_attests(/oracles/delivery.id, "delivered", "true")> true)

// Reputation check
always([+PROCEED] true -> <+oracle_attests(/oracles/reputation.id, "reputation_ok", "true")> true)
```

**Implementation consideration:** 
- Oracle has known public key
- Attestation is signed statement
- Path references the attestation in state

### 5. Threshold: `threshold(n, signers)`

N-of-M multisig without enumerating all combinations:

```modality
// 2-of-3 multisig
always([+EXECUTE] true -> <+threshold(2, /users)> true)
```

**Implementation:** Count valid signatures, check >= n

**Synthesis status:** The predicate language can represent this direct predicate
shape, but threshold-specific model synthesis should stay marked as planned until
there is parser-backed verifier coverage for generated candidates.

### 6. State-Based: `state_equals(path, value)` / `state_exists(path)`

Check contract state:
- "Only proceed if status is 'approved'"
- "Require escrow balance exists"

```modality
always([+RELEASE] true -> <+state_equals(/state/status.text, "approved")> true)
always([+CLAIM] true -> <+state_exists(/state/escrow.json)> true)
```

---

## Predicate Composition

Predicates should compose with formula operators:

```modality
// Time-locked multisig: 2-of-3 OR 1 after deadline
always([+EXECUTE] true -> (
  <+threshold(2, /users)> true |
  (<+signed_by(/users/a.id)> true & <+after(/state/deadline.datetime)> true)
))
```

---

## Implementation Strategy

### WASM Module Interface

Each predicate is a WASM module with standard interface:

```rust
#[no_mangle]
pub fn evaluate(
    commit_data: &[u8],    // The commit being validated
    path_resolver: &impl PathResolver,  // Access to state paths
) -> bool
```

### Predicate Registry

```toml
# predicates.toml
[predicates]
signed_by = "wasm/signed_by.wasm"
before = "wasm/before.wasm"
amount_equals = "wasm/amount_equals.wasm"
threshold = "wasm/threshold.wasm"
```

### Synthesis Integration

When synthesizing from formulas, predicates translate to transition requirements:

```
Formula: always([+PAY] true -> <+amount_gte(100) +signed_by(/users/buyer.id)> true)
    ↓
Transition: state --> paid: +PAY +amount_gte(100) +signed_by(/users/buyer.id)
```

---

## Open Questions

1. **Time source:** How do we get trustless time?
2. **Oracle trust:** How do we establish oracle credibility?
3. **Path resolution:** Static vs dynamic path evaluation?
4. **Gas/cost:** Predicate evaluation cost in consensus?
5. **Composability:** Can predicates call other predicates?

---

## Priority Order

1. `signed_by` ✅ (implemented)
2. `threshold` ✅ (predicate implemented; synthesis heuristic planned)
3. `before`/`after` ✅ (implemented - timestamp predicates)
4. `hash_matches` ✅ (implemented - SHA256, hash equality)
5. `amount_equals` ✅ (implemented - num_equals, num_gte, etc.)
6. `oracle_attests` ✅ (implemented - external attestation)
7. `state_equals` ✅ (implemented - text_equals, bool_equals, etc.)

All core predicates implemented! 🎉
