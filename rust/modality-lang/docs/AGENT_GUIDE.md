# Modality Agent Guide

A practical guide for AI agents to negotiate and execute verifiable contracts.

## Why Modality?

When you make a deal with another agent, how do you know they'll follow through? Modality provides:

1. **Formal verification** - Prove that a contract does what it claims
2. **State tracking** - Know exactly where you are in the contract
3. **Commitment records** - Audit trail of who did what
4. **Evolution** - Upgrade contracts when both parties agree

## Quick Start

### 1. Create a Contract

```rust
use modality_lang::agent::Contract;

// Escrow: you deposit, they deliver, you release payment
let mut contract = Contract::escrow("me", "them");

// Service agreement: offer → accept → deliver → confirm → pay
let mut contract = Contract::service_agreement("provider", "consumer");

// Handshake: both must sign before anything happens
let mut contract = Contract::handshake("party_a", "party_b");

// Atomic swap: neither can cheat, both commit or neither does
let mut contract = Contract::atomic_swap("alice", "bob");
```

### 2. Check Your Options

```rust
// What can I do right now?
let actions = contract.what_can_i_do("me");

for action in actions {
    println!("I can: {} - {}", action.name, action.description);
}
```

### 3. Take Actions

```rust
// Simple action
contract.act("me", "deposit")?;

// Action with custom properties
contract.act_with("me", vec![
    ("deposit", true),
    ("amount_100", true),
])?;
```

### 4. Check Status

```rust
let status = contract.status();
println!("Active: {}", status.is_active);
println!("Complete: {}", status.is_complete);
println!("Actions taken: {}", status.action_count);

// Or get a summary
println!("{}", contract.summary());
// "escrow contract between me and them | State: [flow:deposited] | 1 action taken | ACTIVE"
```

## Contract Types

### Escrow
Best for: Buying/selling with payment protection

```
depositor deposits → deliverer delivers → depositor releases
```

```rust
let mut contract = Contract::escrow("buyer", "seller");
contract.act("buyer", "deposit")?;   // Buyer puts funds in escrow
contract.act("seller", "deliver")?;  // Seller delivers goods/service
contract.act("buyer", "release")?;   // Buyer releases funds
```

### Service Agreement
Best for: Freelance work, API calls, task completion

```
provider offers → consumer accepts → provider delivers → consumer confirms → payment
```

```rust
let mut contract = Contract::service_agreement("worker", "client");
contract.act("worker", "offer")?;
contract.act("client", "accept")?;
contract.act("worker", "deliver")?;
contract.act("client", "confirm")?;
// Payment is automatic after confirm
```

### Atomic Swap
Best for: Trading without trust

```
Both commit their assets → Both can claim only after both committed
```

```rust
let mut contract = Contract::atomic_swap("alice", "bob");
contract.act("alice", "commit_a")?;
contract.act("bob", "commit_b")?;
// Now both can claim
contract.act("alice", "claim")?;
```

### Handshake
Best for: Agreements that require mutual consent first

```
Either can sign first → Both must sign → Contract activates
```

```rust
let mut contract = Contract::handshake("founder_1", "founder_2");
contract.act_with("founder_1", vec![("signed_by_founder_1", true)])?;
contract.act_with("founder_2", vec![("signed_by_founder_2", true)])?;
// Both have agreed, contract is active
```

### Multisig
Best for: Group decisions requiring N-of-M approval

```rust
let mut contract = Contract::multisig(&["alice", "bob", "carol"], 2);
// Any 2 of 3 can approve
contract.act("alice", "vote")?;
contract.act("bob", "vote")?;
// Threshold met
contract.act("alice", "execute")?;
```

## Negotiating Contracts

### Proposing

```rust
use modality_lang::agent::ContractProposal;

// Create a proposal
let proposal = ContractProposal::service(
    "my_agent_id",
    "their_agent_id", 
    "I'll analyze your dataset for 10 tokens"
);

// Send to other party (as JSON)
let json = proposal.to_json()?;
send_to_other_agent(json);
```

### Receiving & Accepting

```rust
// Receive proposal
let json = receive_from_agent();
let proposal = ContractProposal::from_json(&json)?;

// Review it
println!("Type: {}", proposal.proposal_type);
println!("Terms: {:?}", proposal.terms);
println!("From: {}", proposal.proposed_by);

// Accept and create contract
let mut contract = proposal.accept();

// Now both parties can execute the contract
```

## Evolution: Changing Contracts

Contracts can evolve with approval from all parties.

### Adding Rules

```rust
use modality_lang::evolution::{EvolvableContract, Amendment};
use modality_lang::ast::{Transition, Property, PropertySign};

// Wrap a contract for evolution
let mut evolvable = EvolvableContract::new(
    model,
    vec!["alice".to_string(), "bob".to_string()],
    2  // Both must approve
);

// Propose adding a new transition
let mut new_transition = Transition::new("active".to_string(), "paused".to_string());
new_transition.add_property(Property::new(PropertySign::Plus, "PAUSE".to_string()));

let proposal_id = evolvable.propose(
    "alice".to_string(),
    "Add pause functionality".to_string(),
    Amendment::AddTransition {
        part_name: "main".to_string(),
        transition: new_transition,
    },
);

// Both parties approve
evolvable.sign(&proposal_id, "alice", true, None)?;
evolvable.sign(&proposal_id, "bob", true, None)?;

// Execute the change
evolvable.execute(&proposal_id)?;
```

### Replacing the Entire Model

```rust
let new_model = templates::service_agreement("alice", "bob");

let proposal_id = evolvable.propose(
    "alice".to_string(),
    "Upgrade to service agreement model".to_string(),
    Amendment::ReplaceModel { new_model },
);

// Approve and execute...
```

## Serialization

Contracts can be saved and restored:

```rust
// Save
let json = contract.to_json()?;
save_to_file("contract.json", &json);

// Load
let json = load_from_file("contract.json");
let contract = Contract::from_json(&json)?;
```

## Best Practices

1. **Always verify the model** before accepting a proposal
2. **Keep contract JSON** as proof of commitment
3. **Check `what_can_i_do()`** before attempting actions
4. **Use appropriate contract type** - escrow for trades, service for work
5. **Evolution requires trust** - only evolve with parties you trust

## Error Handling

```rust
match contract.act("me", "deposit") {
    Ok(result) => {
        println!("Success! New state: {}", result.new_state);
    }
    Err(e) => {
        println!("Failed: {}", e);
        // Check what you can actually do
        let options = contract.what_can_i_do("me");
        println!("Available actions: {:?}", options);
    }
}
```

## Full Example: Agent Trade

```rust
use modality_lang::agent::{Contract, ContractProposal};

// === AGENT A: Proposing ===
let proposal = ContractProposal::escrow("agent_a", "agent_b");
let proposal_json = proposal.to_json()?;
// Send proposal_json to Agent B...

// === AGENT B: Accepting ===
let received = ContractProposal::from_json(&proposal_json)?;
let mut contract = received.accept();
let contract_json = contract.to_json()?;
// Send contract_json back to Agent A...

// === AGENT A: Depositing ===
let mut contract = Contract::from_json(&contract_json)?;
contract.act("agent_a", "deposit")?;
let contract_json = contract.to_json()?;
// Send updated contract to Agent B...

// === AGENT B: Delivering ===
let mut contract = Contract::from_json(&contract_json)?;
contract.act("agent_b", "deliver")?;
let contract_json = contract.to_json()?;
// Send updated contract to Agent A...

// === AGENT A: Releasing ===
let mut contract = Contract::from_json(&contract_json)?;
contract.act("agent_a", "release")?;

// Contract complete!
assert!(contract.status().is_complete);
```

## Next Steps

- Read `FOR_AGENTS.md` for the trust problem Modality solves
- Read `QUICKSTART.md` for model syntax
- Explore `examples/` for more contract patterns
