---
name: modality
description: Create, sign, and verify Modality contracts for trustless agent cooperation. Use when an agent needs to establish a binding agreement with another agent, delegate a task with enforceable terms, verify a contract's protections, or participate in a multi-party contract. Handles escrow, task delegation, data exchange, multisig, and custom contract patterns.
---

# Modality — Verifiable Contracts for Agents

Create and manage cryptographically enforced contracts between agents. No trust required.

## Quick Start

### Create an escrow contract

```bash
# Create identity
modal id create --path my.passfile

# Create contract
mkdir deal && cd deal
modal c create

# Set up parties
modal c checkout
modal c set /parties/buyer.id $(modal id get --path buyer.passfile)
modal c set /parties/seller.id $(modal id get --path seller.passfile)
```

### Add model + rules

Write `model/default.modality`:
```modality
export default model {
  initial pending
  pending -> funded [+signed_by(/parties/buyer.id)]
  funded -> delivered [+signed_by(/parties/seller.id)]
  delivered -> released [+signed_by(/parties/buyer.id)]
}
```

Write `rules/auth.modality`:
```modality
export default rule {
  starting_at $PARENT
  formula {
    signed_by(/parties/buyer.id) | signed_by(/parties/seller.id)
  }
}
```

Commit: `modal c commit --all --sign buyer.passfile`

## Contract Patterns

Choose a pattern based on your use case:

| Pattern | Use When |
|---------|----------|
| **Escrow** | Buying/selling with payment protection |
| **Task Delegation** | Assigning work with milestone payments |
| **Data Exchange** | Swapping data for payment |
| **Multisig** | Requiring N-of-M approvals |
| **Members Only** | Shared resource with membership control |

See [references/patterns.md](references/patterns.md) for full pattern implementations.

## Reading a Contract (Contract Cards)

When presented with a contract, extract a Card to understand your position:

```json
{
  "my_role": "buyer",
  "my_rights": ["Deposit funds", "Release after delivery", "Dispute"],
  "my_obligations": ["Release or dispute within deadline"],
  "my_protections": ["Seller can't take funds without delivering"],
  "available_actions": ["DEPOSIT"]
}
```

Ask these questions:
1. **What can I do?** → Check `available_actions`
2. **What must I do?** → Check `my_obligations`
3. **Am I protected?** → Check `my_protections`
4. **Can I verify?** → Run model checker on full contract

## Key Concepts

- **Model** = state machine (what CAN happen). Replaceable.
- **Rules** = protection formulas (what MUST be true). Permanent.
- **Predicates** = conditions on transitions: `+signed_by()`, `+modifies()`, `+any_signed()`, `+all_signed()`
- **`+` prefix** = predicate must hold. **`-` prefix** = predicate must NOT hold.
- **Commits** = signed, append-only log entries. Tamper-proof.

## Predicate Reference

| Predicate | Purpose |
|-----------|---------|
| `+signed_by(/path.id)` | Requires specific signer |
| `+any_signed(/path)` | Any member under path signed |
| `+all_signed(/path)` | ALL members under path signed |
| `+modifies(/path)` | Commit writes to path prefix |
| `-modifies(/path)` | Commit must NOT write to path |
| `+threshold(n, /signers)` | N-of-M multisig |
| `+before(/deadline)` | Before timestamp |
| `+after(/deadline)` | After timestamp |

## CLI Commands

```bash
modal c create              # Create new contract
modal c checkout            # Populate working dir from commits
modal c status              # Show contract info + changes
modal c set /path value     # Set state value
modal c commit --all --sign X.passfile  # Commit with signature
modal c log                 # Show commit history
modal id create --path X    # Create ed25519 identity
modal hub start             # Start hub server
modal hub register          # Register with hub
```

## Common Gotchas

1. **Rules use predicates, not action labels** — `+modifies(/path)` not `+MY_ACTION`
2. **Negative predicates required for exclusion** — Without `-modifies(/members)`, a transition CAN modify members
3. **Rule commits need model witness** — Include a model that satisfies the rule
4. **Models replaceable, rules permanent** — Rules are the real enforcement
5. **Members are `.id` files** — `any_signed`/`all_signed` enumerate `*.id` under path
