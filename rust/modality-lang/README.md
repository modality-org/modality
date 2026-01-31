# Modality Language

A verification language for AI agent cooperation. Build verifiable contracts that agents can negotiate, execute, and evolve.

## Why Modality?

When AI agents cooperate, they need more than promises — they need **proofs**. Modality provides:

- **Formal verification**: Prove contracts do what they claim
- **State tracking**: Know exactly where you are in any agreement
- **Path semantics**: Dynamic state addressable via paths like `/members/alice.pubkey`
- **Evolution**: Upgrade contracts with multi-party approval
- **Practical API**: Built for agents to use directly

## Quick Start

### For Agents

```rust
use modality_lang::agent::Contract;

// Create a contract
let mut contract = Contract::escrow("buyer", "seller");

// Check what you can do
let actions = contract.what_can_i_do("buyer");
println!("Available: {:?}", actions);

// Take action
contract.act("buyer", "deposit")?;

// Check status
println!("{}", contract.summary());
// "escrow contract between buyer and seller | State: [flow:deposited] | 1 action | ACTIVE"
```

### Contract Types

```rust
// Basic
let escrow = Contract::escrow("buyer", "seller");
let handshake = Contract::handshake("alice", "bob");
let service = Contract::service_agreement("provider", "consumer");
let swap = Contract::atomic_swap("alice", "bob");
let multisig = Contract::multisig(&["a", "b", "c"], 2);

// Advanced patterns
let protected = Contract::escrow_protected("buyer", "seller", "arbitrator");
let milestones = Contract::milestone("client", "contractor", 3);
let subscription = Contract::subscription("payer", "recipient");
let auction = Contract::auction("seller", 2);
```

### Path Semantics

Contracts maintain typed state addressable via paths:

```rust
use modality_lang::paths::PathValue;

// Set values
contract.post("/escrow/amount.balance", PathValue::Balance(1000))?;
contract.post("/status/note.text", PathValue::Text("Pending".to_string()))?;

// Get values
let pubkey = contract.get_pubkey("/members/alice.pubkey");
let balance = contract.get_balance("/escrow/amount.balance");
```

### Negotiation

```rust
use modality_lang::agent::ContractProposal;

// Create and send proposal
let proposal = ContractProposal::service("me", "them", "10 tokens for analysis");
let json = proposal.to_json()?;

// Other agent accepts
let received = ContractProposal::from_json(&json)?;
let contract = received.accept();
```

### Evolution

```rust
use modality_lang::evolution::{EvolvableContract, Amendment};

// Propose a change
let id = evolvable.propose("alice", "Add pause", Amendment::AddTransition { ... });

// Multi-party approval
evolvable.sign(&id, "alice", true, None)?;
evolvable.sign(&id, "bob", true, None)?;

// Execute when approved
evolvable.execute(&id)?;
```

## CLI

```bash
# Create contract
modality contract create --type escrow --party-a buyer --party-b seller -o contract.json

# Check status
modality contract status --contract contract.json

# See available actions
modality contract actions --contract contract.json --agent buyer

# Take action
modality contract act --contract contract.json --agent buyer --action deposit

# View history
modality contract history --contract contract.json
```

## .modality File Format

```modality
model Escrow {
    init --> deposited: +DEPOSIT +signed_by(/members/buyer.pubkey)
    deposited --> delivered: +DELIVER +signed_by(/members/seller.pubkey)
    delivered --> complete: +RELEASE +signed_by(/members/buyer.pubkey)
}

formula no_release_without_delivery {
    [+RELEASE -DELIVER] false
}

test happy_path {
    contract = Escrow.clone()
    contract.commit(+DEPOSIT +signed_by(/members/buyer.pubkey))
    contract.commit(+DELIVER +signed_by(/members/seller.pubkey))
    contract.commit(+RELEASE +signed_by(/members/buyer.pubkey))
    assert contract.state == "complete"
}
```

## Documentation

- [AGENT_GUIDE.md](docs/AGENT_GUIDE.md) - Comprehensive guide for AI agents
- [PATH_SEMANTICS.md](docs/PATH_SEMANTICS.md) - Path-based contract state
- [QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md) - API quick reference
- [FOR_AGENTS.md](docs/FOR_AGENTS.md) - The trust problem Modality solves

## Features

| Feature | Description |
|---------|-------------|
| **Synthesis** | Generate models from templates (escrow, handshake, etc.) |
| **Verification** | Check temporal logic formulas against models |
| **Evolution** | Propose, approve, and execute contract amendments |
| **Path Store** | Dynamic state with typed paths |
| **Predicates** | WASM-based property verification (ed25519 signatures) |
| **Serialization** | JSON import/export for persistence |

## Architecture

```
modality-lang/
├── src/
│   ├── agent.rs      # High-level agent API
│   ├── runtime.rs    # Contract execution engine
│   ├── evolution.rs  # Contract amendment system
│   ├── paths.rs      # Path-based state store
│   ├── patterns.rs   # Common contract patterns
│   ├── synthesis.rs  # Model generation
│   ├── ast.rs        # Abstract syntax tree
│   ├── grammar.lalrpop # Parser grammar
│   └── model_checker.rs # Formula verification
├── docs/
│   ├── AGENT_GUIDE.md
│   ├── PATH_SEMANTICS.md
│   └── QUICK_REFERENCE.md
├── examples/
│   ├── full-agent-demo.modality
│   ├── evolving-dao.modality
│   └── contract-evolution.modality
└── tests/
    └── integration_tests.rs
```

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --lib agent
cargo test --lib paths
cargo test --lib evolution

# Run integration tests
cargo test --test integration_tests
```

## License

MIT

## Contributing

See the [modality-org/modality](https://github.com/modality-org/modality) repository.
