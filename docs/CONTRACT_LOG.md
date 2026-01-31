# Contract Log: The Append-Only Model

A Modality contract is an **append-only log of signed commits**. Each commit contains **multiactions** and optionally a new **governing model**. The contract's state is derived by replaying the log.

## Core Principles

1. **The log is the source of truth.** There is no separate "state" - state is always derived by replaying commits.

2. **The default model allows everything.** A new contract starts with a maximally permissive model (single node, empty-label self-loop). Rules constrain what's possible.

3. **AddRule is just another action.** Adding a constraint is no different from any other action - it's a signed commit to the log.

4. **Any commit can update the governing model.** As long as the new model satisfies all accumulated rules.

5. **Model proves realizability.** You can't add contradictory rules because no model would satisfy both.

## Data Structures

### Contract

```rust
pub struct ContractLog {
    pub id: String,
    pub commits: Vec<Commit>,
}
```

### Commit

```rust
pub struct Commit {
    pub commit_id: u64,
    pub actions: Vec<Action>,        // Multiactions in this commit
    pub model: Option<Model>,        // Optional: new governing model
    pub signed_by: String,           // Public key of signer
    pub signature: Option<String>,   // Cryptographic signature
    pub timestamp: u64,
}
```

### Action Types

```rust
pub enum Action {
    /// Add a party to the contract
    AddParty { 
        party: String,      // public key
        name: Option<String>,
    },
    
    /// Add a rule (formula) constraint
    AddRule { 
        name: Option<String>,
        formula: Formula,
    },
    
    /// Domain action (state transition)
    Domain { 
        properties: Vec<Property>,  // e.g., +DELIVER, +PAY
    },
    
    /// Finalize negotiation, lock rules
    Finalize,
    
    /// Accept current state
    Accept,
    
    /// Propose a model structure
    ProposeModel {
        model_json: String,
    },
}
```

## Meta-Actions vs Domain Actions

The governing model tracks **domain state**, not contract negotiation.

**Meta-actions** modify the contract structure. They do NOT consume transitions:
- `signed_by X` — who signs this commit
- `model { ... }` — provide/update governing model
- `add_party X` — add a party
- `add_rule { ... }` — add a formula constraint

**Domain actions** execute in the governing model. They MUST be valid transitions:
- `do +ACTION` — execute a domain action

Example:
```modality
// Meta-actions (don't move in model)
commit {
  signed_by A
  model { part flow { init --> done: +DONE } }
  add_party A
  add_rule { eventually(done) }
}

// Domain action (moves init --> done)
commit {
  signed_by A
  do +DONE
}
```

## The Default Model

Every contract starts with a **default governing model**:

```
model Default {
  part flow {
    * --> *   // empty label, self-loop
  }
}
```

This represents "anything goes" - the maximally permissive starting point. Rules then carve out the valid behaviors from the space of all possible actions.

```rust
// New contract has default model
let contract = ContractLog::new("my_contract");
let state = contract.derive_state();

assert_eq!(state.current_model.unwrap().name, "Default");
```

## Adding Rules

`AddRule` adds a formula constraint. The governing model must satisfy ALL accumulated rules.

```rust
// First commit: A adds a rule
contract.commit_with_model(
    "0xAlice...",
    vec![
        Action::AddParty { party: "0xAlice...", name: Some("Alice") },
        Action::AddRule {
            name: Some("AliceProtection"),
            formula: parse("[+DELIVER] eventually(paid | refunded)"),
        },
    ],
    Some(escrow_model),  // Must satisfy AliceProtection
    timestamp,
);

// Second commit: B adds another rule
contract.commit_with_model(
    "0xBob...",
    vec![
        Action::AddParty { party: "0xBob...", name: Some("Bob") },
        Action::AddRule {
            name: Some("BobProtection"),
            formula: parse("[+PAY] eventually(delivered | refunded)"),
        },
    ],
    Some(updated_model),  // Must satisfy BOTH rules
    timestamp,
);
```

## Updating the Governing Model

Any commit can include a new governing model. The new model must satisfy all existing rules.

```rust
// Domain action + model update
contract.commit_with_model(
    "0xAlice...",
    vec![
        Action::Domain { properties: vec![plus("DELIVER")] },
    ],
    Some(refined_model),  // New model, still satisfies all rules
    timestamp,
);

// Just a domain action, no model change
contract.commit(
    "0xBob...",
    vec![
        Action::Domain { properties: vec![plus("PAY")] },
    ],
    timestamp,
);
```

## Validation

Before accepting a commit, validate:

```rust
pub fn validate_commit(
    &self, 
    actions: &[Action], 
    new_model: Option<&Model>
) -> Result<(), String> {
    // 1. Collect all rules (existing + new from this commit)
    let all_rules = existing_rules + new_rules_from_actions;
    
    // 2. Get the model (new if provided, else current, else default)
    let model = new_model.or(current_model).unwrap_or(default_model);
    
    // 3. Model must satisfy ALL rules
    for rule in all_rules {
        if !model_checker.check(model, rule).is_satisfied {
            return Err("Model does not satisfy rule");
        }
    }
    
    Ok(())
}
```

## Derived State

At any point, derive the current state by replaying:

```rust
let state = contract.derive_state();

state.parties         // All parties in the contract
state.rules           // All accumulated rules (formulas)
state.current_model   // Current governing model
state.finalized       // Whether negotiation is locked
state.domain_history  // All domain actions that occurred
```

## Complete Example: Handshake

```modality
contract handshake {

  commit {
    signed_by A
    model {
      part flow {
        init --> a_ready: +A_READY
      }
    }
    add_party A
    add_rule { eventually(a_ready) }
  }

  commit {
    signed_by B
    model {
      part flow {
        init --> a_ready: +A_READY
        a_ready --> done: +B_READY
      }
    }
    add_party B
    add_rule { eventually(done) }
  }

  commit {
    signed_by A
    do +A_READY
  }

  commit {
    signed_by B
    do +B_READY
  }

}
```

Each commit contains:
- `signed_by` — who is signing this commit
- `model` — optional new governing model (must satisfy all rules)
- Actions: `add_party`, `add_rule`, `do +ACTION`

## JSON Representation

```json
{
  "id": "exchange_001",
  "commits": [
    {
      "commit_id": 0,
      "actions": [
        { "type": "AddParty", "party": "0xAlice...", "name": "Alice" },
        { 
          "type": "AddRule", 
          "name": "AliceProtection",
          "formula": { "expression": "[+DELIVER] eventually(paid | refunded)" }
        }
      ],
      "model": { "name": "Exchange", "parts": [...] },
      "signed_by": "0xAlice...",
      "signature": "sig_0",
      "timestamp": 1000
    },
    {
      "commit_id": 1,
      "actions": [
        { "type": "AddParty", "party": "0xBob...", "name": "Bob" },
        { 
          "type": "AddRule", 
          "name": "BobProtection",
          "formula": { "expression": "[+PAY] eventually(delivered | refunded)" }
        }
      ],
      "model": { "name": "Exchange_v2", "parts": [...] },
      "signed_by": "0xBob...",
      "signature": "sig_1",
      "timestamp": 2000
    },
    {
      "commit_id": 2,
      "actions": [{ "type": "Finalize" }],
      "model": null,
      "signed_by": "0xAlice...",
      "signature": "sig_2",
      "timestamp": 3000
    },
    {
      "commit_id": 3,
      "actions": [{ "type": "Domain", "properties": ["+DELIVER"] }],
      "model": null,
      "signed_by": "0xAlice...",
      "signature": "sig_3",
      "timestamp": 4000
    },
    {
      "commit_id": 4,
      "actions": [{ "type": "Domain", "properties": ["+PAY"] }],
      "model": null,
      "signed_by": "0xBob...",
      "signature": "sig_4",
      "timestamp": 5000
    }
  ]
}
```

## Why This Design?

### 1. Auditability
Full history preserved. Every action, every rule, every model change is in the log.

### 2. No Special Phases
Rules can be added anytime (before finalization). There's no separate "negotiation mode" - everything is just commits.

### 3. Incremental Trust
Each party adds their requirements independently. The model must satisfy everyone's rules.

### 4. Deterministic
Same log always produces same derived state. No hidden state.

### 5. Composable
Contracts can reference other contracts' states via paths.

### 6. Self-Enforcing
The model proves the rules are satisfiable. Contradictory rules can't be added because no model would pass validation.

## Formula Syntax Quick Reference

```
// Temporal operators
eventually(P)       // P will be true at some point
always(P)           // P is always true
P until Q           // P holds until Q becomes true
next(P)             // P is true in the next state

// Modal operators (action-labeled)
[+ACTION] P         // After every ACTION, P holds
<+ACTION> P         // After some ACTION, P holds

// Logical operators
P and Q, P & Q      // Conjunction
P or Q, P | Q       // Disjunction
not P, !P           // Negation
P -> Q, P implies Q // Implication

// Propositions
true, false         // Constants
state_name          // Current state matches name
```

## Implementation

See `rust/modality-lang/src/contract_log.rs` for the full implementation.
