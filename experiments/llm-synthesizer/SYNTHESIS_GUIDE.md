# LLM-Enhanced Contract Synthesis Guide

This document captures the approach for using LLMs to synthesize Modality contracts from natural language descriptions.

## Overview

The synthesis process converts natural language contract descriptions into:
1. **Models**: State machines (Labeled Transition Systems)
2. **Rules**: Temporal logic formulas for protection guarantees
3. **Documentation**: Party protections and invariants

## The Synthesis Algorithm

### Step 1: Extract Parties and Roles

From the natural language description, identify:
- **Parties**: Named entities (Alice, Bob, Agent1)
- **Roles**: Functional roles (buyer, seller, coordinator)
- **Trust boundaries**: Who needs protection from whom

Example:
> "Alice wants to buy something from Bob"

Extracted:
- Parties: Alice (buyer), Bob (seller)
- Trust: Alice needs protection against paying without receiving; Bob needs protection against delivering without payment

### Step 2: Identify the Pattern

Map the description to known cooperation patterns:

| Pattern | Keywords | Structure |
|---------|----------|-----------|
| Escrow | deposit, release, hold, payment | Linear: deposit → deliver → release |
| Atomic Swap | trade, exchange, both, neither | Commit-reveal: A commits, B commits, reveal |
| Delegation | delegate, authorize, revoke, on behalf | Branching: delegate → act* → revoke |
| Multisig | approve, N-of-M, quorum, signatures | Fan-in: proposed → signed* → approved |
| Milestone | phase, milestone, deliverable | Sequential: phase1 → paid1 → phase2 → ... |
| Auction | bid, list, winner, highest | Accumulation: listed → bid* → sold → paid |
| Subscription | subscribe, renew, cancel, expire | Cyclic: subscribed ↔ expired |

### Step 3: Design the State Machine

States represent contract conditions:
- **Entry states**: `init`, `pending`, `proposed`
- **Progress states**: `deposited`, `delivered`, `approved`
- **Terminal states**: `complete`, `refunded`, `executed`

Transitions have:
- **Source state**: Where the action starts
- **Target state**: Where it ends
- **Required actions**: `+ACTION_NAME`
- **Forbidden actions**: `-ACTION_NAME`
- **Signature requirements**: `+signed_by(/users/party.id)`

### Step 4: Generate the Model

```modality
model ContractName {
  part flow {
    state1 --> state2: +ACTION +signed_by(/users/party.id)
    state2 --> state3: +ACTION2 +signed_by(/users/other.id)
    state3 --> state3
  }
}
```

Rules:
- State names are **lowercase** with underscores
- Action names are **UPPERCASE** with underscores
- Every state machine needs at least one terminal self-loop

### Step 5: Define Protection Rules

Rules use temporal modal logic:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always ([+RELEASE] implies eventually(<+DELIVER> true))
  }
}
```

Common patterns:
- `always (P)` — invariant, P holds forever
- `[+A] implies Q` — if action A happens, Q must hold
- `eventually(<+A> true)` — action A has happened at some point
- `[<+A>] true` — committed to A (can do A, cannot refuse)

## Validated Examples

The following patterns have been tested and validated:

### Simple Escrow
```modality
model SimpleEscrow {
  part flow {
    pending --> deposited: +DEPOSIT +signed_by(/users/alice.id)
    deposited --> delivered: +DELIVER +signed_by(/users/bob.id)
    delivered --> complete: +RELEASE +signed_by(/users/alice.id)
    complete --> complete
  }
}
```

### Atomic Data Exchange
```modality
model AtomicDataExchange {
  part exchange {
    init --> a_committed: +COMMIT_A +signed_by(/users/agent_a.id)
    init --> b_committed: +COMMIT_B +signed_by(/users/agent_b.id)
    a_committed --> both_committed: +COMMIT_B +signed_by(/users/agent_b.id)
    b_committed --> both_committed: +COMMIT_A +signed_by(/users/agent_a.id)
    both_committed --> a_revealed: +REVEAL_A +signed_by(/users/agent_a.id)
    both_committed --> b_revealed: +REVEAL_B +signed_by(/users/agent_b.id)
    a_revealed --> complete: +REVEAL_B +signed_by(/users/agent_b.id)
    b_revealed --> complete: +REVEAL_A +signed_by(/users/agent_a.id)
    complete --> complete
  }
}
```

### Delegation with Revocation
```modality
model Delegation {
  part authority {
    init --> delegated: +DELEGATE +signed_by(/users/principal.id)
    delegated --> delegated: +ACT_ON_BEHALF +signed_by(/users/delegate.id)
    delegated --> revoked: +REVOKE +signed_by(/users/principal.id)
    revoked --> revoked
  }
}
```

### Multi-signature (2-of-3)
```modality
model Multisig2of3 {
  part approval {
    proposed --> signed_1: +SIGN +signed_by(/users/member1.id)
    proposed --> signed_2: +SIGN +signed_by(/users/member2.id)
    proposed --> signed_3: +SIGN +signed_by(/users/member3.id)
    signed_1 --> approved: +SIGN +signed_by(/users/member2.id)
    signed_1 --> approved: +SIGN +signed_by(/users/member3.id)
    signed_2 --> approved: +SIGN +signed_by(/users/member1.id)
    signed_2 --> approved: +SIGN +signed_by(/users/member3.id)
    signed_3 --> approved: +SIGN +signed_by(/users/member1.id)
    signed_3 --> approved: +SIGN +signed_by(/users/member2.id)
    approved --> executed: +EXECUTE
    executed --> executed
  }
}
```

## Common Pitfalls

1. **Missing terminal self-loops**: Every state machine needs at least one terminal state that loops to itself.

2. **Wrong casing**: States are lowercase, actions are UPPERCASE.

3. **Missing signature requirements**: Use `+signed_by(/path)` for actions that require authentication.

4. **Unreachable states**: Ensure all states are reachable from the initial state.

5. **Dead ends**: Ensure every non-terminal state has at least one outgoing transition.

## Testing Your Model

```bash
# Generate Mermaid diagram
modality model mermaid your_contract.modality

# Check a formula against a model
modality model check your_contract.modality --formula "always(safe)"
```

## Advanced Patterns

### Agent Swarm Coordination
Multiple workers with coordinator aggregation.

### Reputation-Gated Escrow  
Third-party oracle validates reputation before proceeding.

### Dutch Auction
Price decreases over time until first acceptable bid.

See `/experiments/llm-synthesizer/examples/` for validated implementations.

## Next Steps

- Integrate with NL parser for automated pattern detection
- Add rule synthesis from protection descriptions
- Build model checker integration for validation
- Create interactive refinement loop
