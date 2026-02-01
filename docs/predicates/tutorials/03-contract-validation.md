# Tutorial 3: Contract Validation with Predicates

This tutorial shows how to build a complete contract using predicates for validation.

## Scenario: Escrow Contract

Two parties (Alice and Bob) want to create an escrow contract. The contract has:
- A message field for communication
- Status flags for tracking state
- Amount field for the escrowed value

## Contract Structure

```
state/
  message.text        # Communication between parties
  alice_approved.bool # Alice's approval flag
  bob_approved.bool   # Bob's approval flag
  status.text         # "pending", "approved", "released", "disputed"
  amount.text         # Amount in escrow (as string for now)
```

## Defining Validation Rules

### Rule 1: Status Must Be Valid

```modality
rule valid_status {
  starting_at $ROOT
  formula {
    text_equals(/state/status.text, "pending") |
    text_equals(/state/status.text, "approved") |
    text_equals(/state/status.text, "released") |
    text_equals(/state/status.text, "disputed")
  }
}
```

### Rule 2: Messages Must Have Content

```modality
rule message_not_empty {
  starting_at $ROOT
  formula {
    text_not_empty(/state/message.text) &
    text_length_lt(/state/message.text, 1000)
  }
}
```

### Rule 3: Both Must Approve for Release

```modality
rule release_requires_approval {
  starting_at $ROOT
  formula {
    text_equals(/state/status.text, "released") ->
    (bool_is_true(/state/alice_approved.bool) & 
     bool_is_true(/state/bob_approved.bool))
  }
}
```

This reads: "If status is 'released', then both Alice and Bob must have approved."

## Checking Rule Compatibility

Before committing these rules, we check they don't contradict:

### Check 1: message_not_empty predicates

```rust
let input = CorrelationInput {
    params: json!({}),  // text_not_empty has no params
    other_rules: vec![
        RuleContext {
            predicate: "text_length_lt".to_string(),
            params: json!({"length": 1000}),
        }
    ],
};
let result = text_not_empty::correlate(&input);
// Result: satisfiable=true
// Formula: "text_not_empty($path) -> text_length_gt($path, 0)"
```

These are compatible - a non-empty string must have length > 0, which can also be < 1000.

### Check 2: Approval flags

```rust
// If Alice sets alice_approved=true and rule says bool_is_true
let input = CorrelationInput {
    params: json!({}),
    other_rules: vec![
        RuleContext {
            predicate: "bool_equals".to_string(),
            params: json!({"expected": true}),
        }
    ],
};
let result = bool_is_true::correlate(&input);
// Result: satisfiable=true
// Formula: "bool_is_true($path) <-> bool_equals($path, true)"
```

## Complete Validation Flow

```
1. Alice creates contract with initial rules
   ↓
2. System runs correlate on all rules
   ↓
3. If satisfiable=false anywhere, reject
   ↓
4. Bob adds his rules
   ↓
5. System runs correlate including Bob's rules
   ↓
6. If satisfiable=false, reject Bob's rules
   ↓
7. Contract is ready - all rules are compatible
```

## Example: Detecting a Bad Rule

What if Bob tries to add a contradictory rule?

```modality
// Bob's bad rule - status must be "canceled"
rule bob_wants_canceled {
  formula { text_equals(/state/status.text, "canceled") }
}
```

This contradicts `valid_status` which only allows "pending", "approved", "released", "disputed".

Correlate detects this:

```json
{
  "formulas": [
    "!(text_equals($path, \"canceled\") & text_equals($path, \"pending\"))",
    "!(text_equals($path, \"canceled\") & text_equals($path, \"approved\"))",
    "!(text_equals($path, \"canceled\") & text_equals($path, \"released\"))",
    "!(text_equals($path, \"canceled\") & text_equals($path, \"disputed\"))"
  ],
  "satisfiable": false,
  "gas_used": 20
}
```

Bob's rule is rejected before it can cause problems.

## Running the Example

```bash
cd modality/rust
cargo run -p modal-wasm-validation --example escrow_validation
```

## Key Takeaways

1. **Predicates validate data** - Each predicate checks one condition
2. **Correlate checks compatibility** - Before committing rules, verify they can coexist
3. **Formulas express relationships** - The generated formulas document exactly how rules interact
4. **Early detection** - Catch contradictions at rule-commit time, not at runtime

## What's Next?

- Add more predicates for your use case (numbers, dates, etc.)
- Build a rule validator that automatically runs correlate
- Create a UI that shows formula explanations to users

## Full Code Example

See `examples/escrow_validation.rs` for the complete working code.
