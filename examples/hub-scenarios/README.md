# Hub Scenarios: Multi-Party Contract Examples

Real-world examples of multiple agents using a hub to coordinate contracts.

## Scenarios

| Scenario | Parties | Use Case |
|----------|---------|----------|
| [escrow-3party](escrow-3party.md) | Buyer, Seller, Arbiter | Purchase with dispute resolution |
| [treasury-multisig](treasury-multisig.md) | 5 Board Members | 3-of-5 approval for payments |
| [service-agreement](service-agreement.md) | Client, Provider | Milestone-based project |
| [agent-swarm](agent-swarm.md) | Coordinator + Workers | Task distribution & rewards |

## Key Patterns

### 1. Setup Phase
```bash
# One party creates the contract
modal hub create "Contract Name"

# Add model and initial state
modal c commit --all -m "Initialize"
modal c push --remote hub

# Invite other parties
modal hub grant <contract_id> <identity_id> write
```

### 2. Join Phase
```bash
# Other parties clone and add their identity
modal c create --contract-id <id>
modal c pull --remote hub
echo 'ed25519:my_key' > state/parties/me.id
modal c commit --all -m "Join contract"
modal c push --remote hub
```

### 3. Execution Phase
```bash
# Pull latest state
modal c pull --remote hub

# Take an action (validated against model)
modal c commit --action '{"method":"ACTION","action":"DO_THING"}' --sign me.passfile
modal c push --remote hub
```

## Commit Types

| Type | Purpose | Example |
|------|---------|---------|
| `POST` | Add/update data | Party registration, state files |
| `RULE` | Add model/rules | `rules/escrow.modality` |
| `ACTION` | Domain action | `DEPOSIT`, `APPROVE`, `SUBMIT` |

## Validation

The hub validates all ACTION commits:
1. **State check** - Action allowed from current state?
2. **Guard check** - Required signatures present?
3. **Threshold check** - Multisig requirements met?

## Common Patterns

### Signature Guard
```modality
state1 -> state2 : ACTION [+signed_by(/parties/alice.id)]
```

### Multi-Signature
```modality
state1 -> state2 : ACTION [+signed_by(/parties/a.id) +signed_by(/parties/b.id)]
```

### Either-Or Signature
```modality
state1 -> state2 : ACTION [+signed_by(/parties/a.id) | +signed_by(/parties/b.id)]
```

### Threshold (n-of-m)
```modality
state1 -> state2 : ACTION [+threshold(3, /approvals, /members)]
```

## Running the Examples

```bash
# Start a hub
modal hub start --detach

# Register identities for each party
modal hub register --output alice-creds.json
modal hub register --output bob-creds.json

# Follow the scenario steps...
```
