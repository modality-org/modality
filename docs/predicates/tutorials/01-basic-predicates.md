# Tutorial 1: Basic Predicate Usage

This tutorial shows how to use predicates to validate contract data.

## What Are Predicates?

Predicates are functions that check if data meets certain conditions. In Modality, predicates work on **paths** - locations in your contract's state.

For example, if your contract has:
```
state/
  message.text     # contains "hello world"
  approved.bool    # contains true
```

You can use predicates to enforce rules about these values.

## Text Predicates

### Checking Exact Values

```modality
rule message_must_be_greeting {
  formula {
    text_equals(/state/message.text, "hello")
  }
}
```

This rule passes only if `/state/message.text` equals exactly `"hello"`.

### Checking Patterns

```modality
rule message_must_start_with_hi {
  formula {
    text_starts_with(/state/message.text, "hi")
  }
}
```

### Combining Length Constraints

```modality
rule message_length_valid {
  formula {
    text_length_gt(/state/message.text, 5) &
    text_length_lt(/state/message.text, 100)
  }
}
```

This ensures the message is between 6 and 99 characters.

## Bool Predicates

### Simple Boolean Checks

```modality
rule must_be_approved {
  formula {
    bool_is_true(/state/approved.bool)
  }
}
```

### Using bool_equals

```modality
rule check_flag {
  formula {
    bool_equals(/state/enabled.bool, true)
  }
}
```

## Evaluate vs Correlate

Every predicate has two functions:

### evaluate()

Checks if the predicate holds for actual data:

```json
// Input
{
  "data": { "value": "hello", "expected": "hello" },
  "context": { "contract_id": "abc", "block_height": 100, "timestamp": 1234567890 }
}

// Output
{
  "valid": true,
  "gas_used": 10,
  "errors": []
}
```

### correlate()

Generates formulas when combined with other predicates:

```json
// Input
{
  "params": { "expected": "hello" },
  "other_rules": [
    { "predicate": "text_length_eq", "params": { "length": 5 } }
  ]
}

// Output
{
  "formulas": [
    "text_equals($path, \"hello\") -> text_length_eq($path, 5)"
  ],
  "satisfiable": true,
  "gas_used": 20
}
```

The formula `A -> B` means "if A holds, then B must also hold". Since `"hello"` has length 5, these predicates are compatible.

## Try It Yourself

Run the example:

```bash
cargo run -p modal-wasm-validation --example correlate_demo
```

## Next Steps

- [Tutorial 2: Combining Predicates](./02-combining-predicates.md) - Learn how predicates interact
- [Tutorial 3: Contract Validation](./03-contract-validation.md) - Build complete validation rules
