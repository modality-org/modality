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
always([+DELIVER] implies before(/state/deadline.datetime))

// Refund only after deadline
always([+REFUND] implies after(/state/deadline.datetime))
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
always([+DEPOSIT] implies amount_equals(/state/price.json))

// Minimum payment
always([+PAY] implies amount_gte(/state/minimum.json))
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
always([+REVEAL_A] implies preimage_of(/state/commitment_a.hash))

// Hash time-locked contracts
always([+CLAIM] implies hash_matches(/state/secret.hash))
```

**Implementation:** SHA256 or Blake3, committed value in state

### 4. Oracle-Based: `oracle_attests(condition)`

External verification:
- "Oracle confirms goods were delivered"
- "Reputation service approves seller"

```modality
// Oracle attestation
always([+RELEASE] implies oracle_attests(/oracles/delivery.bool))

// Reputation check
always([+PROCEED] implies oracle_attests(/oracles/reputation_ok.bool))
```

**Implementation consideration:** 
- Oracle has known public key
- Attestation is signed statement
- Path references the attestation in state

### 5. Threshold: `threshold(n, signers)`

N-of-M multisig without enumerating all combinations:

```modality
// 2-of-3 multisig
always([+EXECUTE] implies threshold(2, [
  /users/alice.id,
  /users/bob.id,
  /users/carol.id
]))
```

**Implementation:** Count valid signatures, check >= n

### 6. State-Based: `state_equals(path, value)` / `state_exists(path)`

Check contract state:
- "Only proceed if status is 'approved'"
- "Require escrow balance exists"

```modality
always([+RELEASE] implies state_equals(/state/status.text, "approved"))
always([+CLAIM] implies state_exists(/state/escrow.json))
```

---

## Predicate Composition

Predicates should compose with formula operators:

```modality
// Time-locked multisig: 2-of-3 OR 1 after deadline
always([+EXECUTE] implies (
  threshold(2, [/users/a.id, /users/b.id, /users/c.id]) |
  (signed_by(/users/a.id) & after(/state/deadline.datetime))
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
Formula: always([+PAY] implies (amount_gte(100) & signed_by(buyer)))
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

1. `signed_by` ✅ (already implemented)
2. `threshold` — essential for multisig
3. `before`/`after` — essential for deadlines
4. `hash_matches` — essential for atomic swaps
5. `amount_equals` — essential for payments
6. `oracle_attests` — for external verification
7. `state_equals` — for complex conditions
