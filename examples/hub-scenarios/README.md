# Hub Scenarios: Multi-Party Contract Examples

Real-world examples of multiple agents using a hub to coordinate contracts.

## Scenarios

| Scenario | Parties | Use Case |
|----------|---------|----------|
| [escrow-3party](escrow-3party.md) | Buyer, Seller, Arbiter | Purchase with dispute resolution |
| [treasury-multisig](treasury-multisig.md) | 5 Board Members | 3-of-5 approval for payments |
| [service-agreement](service-agreement.md) | Client, Provider | Milestone-based project |
| [agent-swarm](agent-swarm.md) | Coordinator + Workers | Task distribution & rewards |
| [members-only](members-only.md) | Members | Unanimous consent to change the membership |
| [bank-deposits](bank-deposits.md) | Admin + Account holders | Accounts with deposits and withdrawals (JavaScript SDK) |

## Key Patterns

Each scenario runs in order from an empty directory against a local hub.

### 1. Setup
```bash
# A hub for the demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &

# One party creates the contract: parties, model and rules, in one commit
modal c create --dir mine && cd mine
modal c set-named-id /parties/alice.id ../alice.passfile
modal add-rule --name alice_signs 'always([-signed_by(/parties/alice.id)] false)'
modal c commit --all --sign ../alice.passfile -m "Setup"

# The first push names the hub URL and saves it as `origin`
modal c push --remote http://127.0.0.1:8080/contracts/$(modal c id)
```

### 2. Join
```bash
# Anyone copies the contract from its URL
modal c pull http://127.0.0.1:8080/contracts/<contract-id> --dir theirs
```

### 3. Execution
```bash
modal c pull
modal c commit --path /escrow/deposited.bool --value true --sign ../alice.passfile -m "Deposit"
modal c push
```

Every copy checks each commit against the contract's model and rules before
it is made, and the hub checks it again on push: signatures, then the rules.

## Common Patterns

The rules speak of what a commit writes and who signed it.

### Signature guard
```modality
always([+modifies(/escrow/released.bool) -signed_by(/parties/alice.id)] false)
```

### Multi-signature
```modality
always(([+modifies(/treasury) -signed_by(/parties/a.id)] false) & ([+modifies(/treasury) -signed_by(/parties/b.id)] false))
```

### Either-or signature
```modality
always([+modifies(/escrow/released.bool) -signed_by(/parties/a.id) -signed_by(/parties/b.id)] false)
```

### Threshold (n-of-m)
```modality
always([+modifies(/payments) -threshold("3", /board)] false)
```

### Order of steps
```modality
always([+modifies(/escrow/delivered.bool) -bool_true(/escrow/deposited.bool)] false)
```

A model that meets these rules gives each step its own edge, and says which
paths the edge leaves alone: labels are open, so an edge that does not rule a
write out may be taken by a commit that makes it.

## Running the Examples

Each scenario file is a script in order: run its `bash` blocks one after
another from an empty directory, with `modal` on your `PATH`. The last block
stops the hub.
