---
sidebar_position: 3
title: State Machines
---

# State Machines (Models)

A **model** defines the allowed behaviors as a labeled transition system (LTS).

## Basic Structure

```modality
model escrow {
  states { pending, funded, delivered, released, refunded }
  initial pending
  terminal released, refunded
  
  transition DEPOSIT: pending -> funded
    +signed_by(/parties/buyer.id)
  
  transition DELIVER: funded -> delivered
    +signed_by(/parties/seller.id)
  
  transition RELEASE: delivered -> released
    +signed_by(/parties/buyer.id)
}
```

## Components

| Component | Description |
|-----------|-------------|
| `states` | All possible states the contract can be in |
| `initial` | The starting state |
| `terminal` | End states (self-loop implied) |
| `transition` | A labeled edge: `ACTION: from -> to` |
| `+predicate` | Conditions that must hold for the transition |

## Visualization

The escrow model above looks like:

```
          DEPOSIT           DELIVER           RELEASE
pending ──────────► funded ─────────► delivered ─────────► released
                      │                                        ▲
                      │                                        │
                      └────────────────────────────────────────┘
                                      REFUND
                                         ▼
                                     refunded
```

## Why State Machines?

State machines provide:

1. **Finite behaviors** — Only defined transitions are possible
2. **Verifiability** — We can check if a formula holds
3. **Explicitness** — No hidden states or transitions
4. **Composability** — Models can be analyzed together
