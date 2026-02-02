---
sidebar_position: 2
title: Model Syntax
---

# Model Syntax

Models define **state machines** — the allowed behaviors of a contract.

## Basic Structure

```modality
model <name> {
  states { <state1>, <state2>, ... }
  initial <state>
  terminal <state1>, <state2>, ...
  
  transition <ACTION>: <from_state> -> <to_state>
    +<predicate1>
    +<predicate2>
}
```

## Complete Example

```modality
model escrow {
  states { pending, funded, delivered, released, refunded, disputed }
  initial pending
  terminal released, refunded
  
  // Buyer deposits funds
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/buyer.id)
    +num_gte(/deposit/amount.num, /terms/price.num)
  
  // Seller delivers goods
  transition DELIVER: funded -> delivered
    +signed_by(/parties/seller.id)
  
  // Buyer releases payment
  transition RELEASE: delivered -> released
    +signed_by(/parties/buyer.id)
}
```

## State Declarations

```modality
// List of all states
states { state1, state2, state3 }

// Initial state (required)
initial state1

// Terminal states (optional) - self-loop implied
terminal state2, state3
```

## Transitions

```modality
// Basic transition
transition ACTION_NAME: from_state -> to_state

// With predicates (all must be satisfied)
transition ACTION_NAME: from_state -> to_state
  +predicate1(args)
  +predicate2(args)

// Multiple transitions with same action (non-deterministic)
transition DISPUTE: funded -> disputed
transition DISPUTE: delivered -> disputed
```

## Comments

```modality
// Single-line comment

/* 
   Multi-line
   comment
*/

model escrow {
  // States for the escrow workflow
  states { pending, funded }
}
```
