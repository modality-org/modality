# Contract Log: The Append-Only Model

A Modality contract is an **append-only log of signed commits**. Each commit contains **multiactions**. The contract's state is derived by replaying the log.

## Core Concepts

### The Log

```
Contract {
  id: "unique_id",
  commits: [
    Commit { actions: [...], signed_by: A, timestamp: t0 },
    Commit { actions: [...], signed_by: B, timestamp: t1 },
    ...
  ]
}
```

The log is the source of truth. There is no separate "state" - state is always derived by replaying commits.

### Action Types

| Action | Purpose |
|--------|---------|
| `AddParty` | Add a party (public key) to the contract |
| `AddRule` | Add a formula constraint |
| `Domain` | Domain-specific action (e.g., +DELIVER, +PAY) |
| `Finalize` | Lock the negotiation phase |
| `Accept` | Accept current state |
| `ProposeModel` | Optionally propose explicit state machine |

### Adding Rules

`AddRule` is just another action. It adds a formula that all future actions must satisfy.

**Critical: AddRule must include a state machine that satisfies the formula AND all prior rules.** Otherwise the commit is rejected.

```json
{
  "type": "AddRule",
  "name": "MyProtection",
  "formula": {
    "expression": { "Eventually": { "Prop": "paid" } }
  },
  "model": {
    "name": "Exchange",
    "parts": [{
      "name": "flow",
      "transitions": [
        { "from": "init", "to": "paid" }
      ]
    }]
  }
}
```

The model proves realizability. You can't add contradictory rules because no model would satisfy both.

Rules accumulate. Each domain action is validated against ALL rules.

## Example: Two-Party Exchange

### Commit 0: Alice creates contract

```json
{
  "commit_id": 0,
  "actions": [
    { "type": "AddParty", "party": "0xAlice...", "name": "Alice" },
    { 
      "type": "AddRule",
      "name": "AliceProtection",
      "formula": "[+DELIVER] eventually(paid | refunded)"
    }
  ],
  "signed_by": "0xAlice..."
}
```

Alice joins and states her requirement: if she delivers, she eventually gets paid or refunded.

### Commit 1: Bob joins

```json
{
  "commit_id": 1,
  "actions": [
    { "type": "AddParty", "party": "0xBob...", "name": "Bob" },
    {
      "type": "AddRule", 
      "name": "BobProtection",
      "formula": "[+PAY] eventually(delivered | refunded)"
    }
  ],
  "signed_by": "0xBob..."
}
```

Bob joins and states his requirement: if he pays, he eventually gets delivery or refund.

### Commit 2: Finalize negotiation

```json
{
  "commit_id": 2,
  "actions": [
    { "type": "Finalize" }
  ],
  "signed_by": "0xAlice..."
}
```

### Commit 3: Alice delivers

```json
{
  "commit_id": 3,
  "actions": [
    { "type": "Domain", "properties": ["+DELIVER"] }
  ],
  "signed_by": "0xAlice..."
}
```

This action is validated against both rules. It satisfies Alice's rule (delivery occurred). Bob's rule now requires payment to eventually happen.

### Commit 4: Bob pays

```json
{
  "commit_id": 4,
  "actions": [
    { "type": "Domain", "properties": ["+PAY"] }
  ],
  "signed_by": "0xBob..."
}
```

Both rules are now satisfied. Contract complete.

## Derived State

At any point, derive the current state by replaying:

```rust
let state = contract.derive_state();

state.parties    // ["0xAlice...", "0xBob..."]
state.rules      // [AliceProtection, BobProtection]
state.finalized  // true
state.domain_history  // [(3, [+DELIVER]), (4, [+PAY])]
```

## Validation

Before accepting a commit, validate:

1. **Signature valid**: Commit is signed by a party in the contract
2. **Rules satisfied**: Domain actions don't violate any accumulated formulas
3. **Ordering valid**: Action is valid given current derived state

```rust
contract.validate_commit(&new_actions)?;
contract.commit(signed_by, new_actions, timestamp);
```

## Why This Design?

1. **Auditability**: Full history preserved
2. **No special phases**: Rules can be added anytime (before finalization)
3. **Incremental trust**: Each party adds their requirements
4. **Deterministic**: Same log always produces same state
5. **Composable**: Contracts can reference other contracts' states

## Implementation

```rust
use modality_lang::{ContractLog, Action, Formula};

// Create contract
let mut contract = ContractLog::new("my_contract".to_string());

// Alice creates and adds rule
contract.commit(
    "0xAlice...".to_string(),
    vec![
        Action::AddParty { party: "0xAlice...".to_string(), name: Some("Alice".to_string()) },
        Action::AddRule { 
            name: Some("AliceProtection".to_string()),
            formula: parse_formula("[+DELIVER] eventually(paid | refunded)")
        },
    ],
    timestamp(),
);

// Bob joins
contract.commit(
    "0xBob...".to_string(),
    vec![
        Action::AddParty { party: "0xBob...".to_string(), name: Some("Bob".to_string()) },
        Action::AddRule { 
            name: Some("BobProtection".to_string()),
            formula: parse_formula("[+PAY] eventually(delivered | refunded)")
        },
    ],
    timestamp(),
);

// Execute
contract.commit("0xAlice...".to_string(), vec![Action::Domain { properties: vec![plus("DELIVER")] }], timestamp());
contract.commit("0xBob...".to_string(), vec![Action::Domain { properties: vec![plus("PAY")] }], timestamp());

// Check state
let state = contract.derive_state();
assert!(state.rules.len() == 2);
assert!(state.domain_history.len() == 2);
```
