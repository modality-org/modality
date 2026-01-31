# Modality Quick Reference

## Create a Contract

```rust
use modality_lang::agent::Contract;

// Built-in types
let escrow = Contract::escrow("buyer", "seller");
let handshake = Contract::handshake("alice", "bob");
let service = Contract::service_agreement("provider", "consumer");
let swap = Contract::atomic_swap("alice", "bob");
let multisig = Contract::multisig(&["a", "b", "c"], 2);
let cooperation = Contract::mutual_cooperation("a", "b");

// Advanced patterns
let protected = Contract::escrow_protected("buyer", "seller", "arbitrator");
let milestones = Contract::milestone("client", "contractor", 3);
let subscription = Contract::subscription("payer", "recipient");
let auction = Contract::auction("seller", 2);
```

## Check What You Can Do

```rust
let actions = contract.what_can_i_do("alice");
for action in &actions {
    println!("  {} - {}", action.name, action.description);
}

// Convenience methods
let steps = contract.next_steps();      // Human-readable guidance
let who = contract.who_can_act();       // Who can act right now
let my_turn = contract.is_turn("alice"); // Is it my turn?
```

## Take Actions

```rust
// Simple action
contract.act("alice", "deposit")?;

// With custom properties
contract.act_with("alice", vec![
    ("deposit", true),
    ("amount_100", true),
])?;
```

## Path Store (Dynamic State)

```rust
use modality_lang::paths::PathValue;

// Set values
contract.post("/escrow/amount.balance", PathValue::Balance(1000))?;
contract.post("/status/note.text", PathValue::Text("Pending".to_string()))?;

// Get values
let pubkey = contract.get_pubkey("/members/alice.pubkey");
let balance = contract.get_balance("/escrow/amount.balance");
let exists = contract.path_exists("/status/note.text");
```

## Status & History

```rust
// Quick summary
println!("{}", contract.summary());
// "escrow contract between alice and bob | State: [flow:deposited] | 1 action taken | ACTIVE"

// Detailed status
let status = contract.status();
println!("Active: {}", status.is_active);
println!("Complete: {}", status.is_complete);
println!("Actions: {}", status.action_count);

// History
for entry in contract.history() {
    println!("#{} {} by {}", entry.sequence, entry.action, entry.by);
}
```

## Negotiation (Proposals)

```rust
use modality_lang::agent::ContractProposal;

// Create proposal
let proposal = ContractProposal::service("me", "them", "10 tokens for analysis");
let json = proposal.to_json()?;
// Send to other agent...

// Receive and accept
let received = ContractProposal::from_json(&json)?;
let contract = received.accept();
```

## Serialization

```rust
// Save
let json = contract.to_json()?;
std::fs::write("contract.json", &json)?;

// Load
let json = std::fs::read_to_string("contract.json")?;
let contract = Contract::from_json(&json)?;
```

## Evolution (Amendments)

```rust
use modality_lang::evolution::{EvolvableContract, Amendment};

let mut evolvable = EvolvableContract::new(model, governors, threshold);

// Propose change
let id = evolvable.propose("alice", "Add pause", Amendment::AddTransition { ... });

// Approve
evolvable.sign(&id, "alice", true, None)?;
evolvable.sign(&id, "bob", true, None)?;

// Execute
evolvable.execute(&id)?;
```

## CLI Commands

```bash
# Create contract
modality contract create --type escrow --party-a buyer --party-b seller -o contract.json

# Propose
modality contract propose --type service --from me --to them --terms "10 tokens" -o proposal.json

# Accept proposal
modality contract accept --proposal proposal.json -o contract.json

# Check status
modality contract status --contract contract.json

# See available actions
modality contract actions --contract contract.json --agent alice

# Take action
modality contract act --contract contract.json --agent alice --action deposit

# View history
modality contract history --contract contract.json
```

## Path Types

| Extension | Type | Example |
|-----------|------|---------|
| `.text` | String | `/status/note.text` |
| `.int` | i64 | `/counts/total.int` |
| `.bool` | bool | `/flags/approved.bool` |
| `.balance` | u64 | `/balances/alice.balance` |
| `.pubkey` | String | `/members/alice.pubkey` |
| `.set` | Vec | `/tags/labels.set` |
| `.list` | Vec | `/history/events.list` |
| `.json` | JSON | `/config/settings.json` |

## Contract Flow Patterns

### Escrow
```
init → deposited → delivered → complete
       ↓ (timeout)
       reclaim → complete
```

### Service Agreement
```
init → offered → accepted → delivered → confirmed → complete
```

### Atomic Swap
```
init → a_committed → both_committed → complete
init → b_committed ↗
```

### Milestone
```
agreed → m1_funded → m1_delivered → m1_confirmed →
       → m2_funded → m2_delivered → m2_confirmed → ...
```
