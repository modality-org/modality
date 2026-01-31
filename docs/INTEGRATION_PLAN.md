# Modality + Modal Integration Plan

## Current State

### modality-lang (verification layer)
- **Parser**: Brace-based syntax for models, formulas, actions
- **AST**: Model, Part, Transition, Property, Formula (HML/modal-mu-calc)
- **Runtime**: ContractInstance, commit/action flow, state tracking
- **Contract Log**: Append-only log, AddRule/Domain/Finalize actions
- **Crypto**: ed25519 signing/verification via crypto.rs
- **Model Checker**: Formula verification against state machines
- **Synthesis**: Generate models from patterns (escrow, handshake, etc.)

### modal (network layer)
- **Network**: libp2p gossip, request-response, Kademlia DHT
- **Storage**: RocksDB via DatastoreManager (contracts, commits, assets)
- **Consensus**: Narwhal/Shoal BFT for ordering
- **Contract Processor**: Handles commits, WASM predicates, asset state
- **CLI**: contract create/commit/push/pull/status

## Integration Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Agent / CLI                              │
├─────────────────────────────────────────────────────────────┤
│                   modality-lang                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐    │
│  │   Parser    │ │  Synthesis  │ │    Model Checker    │    │
│  └─────────────┘ └─────────────┘ └─────────────────────┘    │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐    │
│  │   Runtime   │ │ Contract Log│ │   Crypto (ed25519)  │    │
│  └─────────────┘ └─────────────┘ └─────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                   modal-validator                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │            ModalityContractProcessor                 │    │
│  │   - Validate commits against formulas                │    │
│  │   - Verify signatures via signed_by predicate        │    │
│  │   - Track contract state                             │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                     modal-node                               │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐   │
│  │  libp2p   │ │ Consensus │ │  Storage  │ │   Sync    │   │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Phase 1: Contract Format Bridge

**Goal**: Store modality-lang contracts in modal storage format

### 1.1 Contract Initialization
```rust
// New commit type: InitModality
{
  "type": "init_modality",
  "version": "0.1",
  "parties": ["alice_pubkey", "bob_pubkey"],
  "model": null  // Optional initial model
}
```

### 1.2 Rule Addition (AddRule commits)
```rust
// Commit that adds a formula constraint
{
  "type": "add_rule", 
  "formula": "[+DELIVER] eventually(paid | refunded)",
  "signed_by": "alice_pubkey",
  "signature": "..."
}
```

### 1.3 Domain Actions
```rust
// Commit that performs a domain action
{
  "type": "domain_action",
  "action": "+PAY",
  "payload": {"amount": 100, "recipient": "bob"},
  "signed_by": "alice_pubkey", 
  "signature": "..."
}
```

## Phase 2: Validation Integration

**Goal**: Validate commits against accumulated formulas

### 2.1 ModalityContractProcessor
```rust
// In modal-validator/src/modality_processor.rs
pub struct ModalityContractProcessor {
    datastore: Arc<Mutex<DatastoreManager>>,
    // Cache of contract_id -> ContractLog
    contract_logs: HashMap<String, ContractLog>,
}

impl ModalityContractProcessor {
    /// Process a modality-specific commit
    pub async fn process_modality_commit(
        &mut self,
        contract_id: &str,
        commit: &ModalityCommit,
    ) -> Result<Vec<StateChange>> {
        let log = self.get_or_create_log(contract_id).await?;
        
        match &commit.action {
            ModalityAction::AddRule { formula, signed_by, signature } => {
                // 1. Verify signature
                self.verify_signature(signed_by, &commit.hash(), signature)?;
                // 2. Parse formula
                let parsed = parse_formula(formula)?;
                // 3. Add to log
                log.add_rule(signed_by, parsed)?;
            }
            ModalityAction::Domain { action, payload, signed_by, signature } => {
                // 1. Verify signature
                self.verify_signature(signed_by, &commit.hash(), signature)?;
                // 2. Check if action is allowed by ALL rules
                log.validate_action(action, payload)?;
                // 3. Apply action
                log.apply_action(action, payload)?;
            }
            ModalityAction::Finalize => {
                log.finalize()?;
            }
        }
        
        Ok(vec![StateChange::ModalityCommit { ... }])
    }
}
```

### 2.2 Integration with ShoalValidator
```rust
// In modal-validator/src/shoal_validator.rs
// Add to process_certificate:

for tx in &transactions {
    if let Ok(modality_commit) = parse_modality_commit(&tx.data) {
        self.modality_processor
            .process_modality_commit(&modality_commit.contract_id, &modality_commit)
            .await?;
    }
}
```

## Phase 3: Agent API

**Goal**: Simple API for agents to negotiate and execute contracts

### 3.1 High-Level Agent API
```rust
// Agent creates a protected escrow
let contract = ModalityContract::new(node_addr)?;

// Alice proposes her protection
contract.propose_rule(
    "[+DELIVER] eventually(paid | refunded)",
    &alice_keypair,
).await?;

// Bob proposes his protection  
contract.propose_rule(
    "[+PAY] eventually(goods | refund)",
    &bob_keypair,
).await?;

// Both finalize
contract.finalize(&alice_keypair).await?;
contract.finalize(&bob_keypair).await?;

// Now execute domain actions
contract.act("+DELIVER", json!({"tracking": "..."}), &alice_keypair).await?;
contract.act("+PAY", json!({"amount": 100}), &bob_keypair).await?;
```

### 3.2 CLI Commands
```bash
# Create modality contract
modal modality create --parties alice.pub,bob.pub

# Add rule (as alice)
modal modality add-rule \
  --contract <id> \
  --formula "[+DELIVER] eventually(paid | refunded)" \
  --sign-with alice.key

# Execute action
modal modality act \
  --contract <id> \
  --action "+PAY" \
  --payload '{"amount": 100}' \
  --sign-with bob.key

# Check status
modal modality status --contract <id>
```

## Phase 4: Full Integration

### 4.1 Wire Protocol
- Add Modality-specific request/response types to libp2p protocol
- `ModalityPropose`, `ModalityAccept`, `ModalityAct`

### 4.2 Persistence
- Store contract logs in RocksDB
- Replay on node restart
- Sync logs between nodes

### 4.3 Model Synthesis
- `modal modality synthesize --pattern escrow --parties alice,bob`
- Generate and deploy pre-verified patterns

## Implementation Order

1. **Week 1**: Contract format bridge (Phase 1)
   - Define commit types in modal-datastore
   - Add modality-lang dependency to modal-validator
   - Basic parsing of modality commits

2. **Week 2**: Validation integration (Phase 2)
   - ModalityContractProcessor
   - Signature verification via crypto.rs
   - Formula validation against actions

3. **Week 3**: Agent API (Phase 3)
   - High-level Rust API
   - CLI commands
   - Example contracts

4. **Week 4**: Polish & Testing (Phase 4)
   - Integration tests
   - Multi-node testing
   - Documentation

## Files to Create/Modify

### New Files
- `modal-validator/src/modality_processor.rs`
- `modal-validator/src/modality_types.rs`
- `modal/src/cmds/modality.rs` (CLI)
- `modality-lang/src/network.rs` (network-aware contract)

### Modified Files
- `modal-validator/Cargo.toml` (add modality-lang dep)
- `modal-validator/src/shoal_validator.rs` (hook processor)
- `modal-datastore/src/models/mod.rs` (commit types)
- `modal/src/main.rs` (add modality subcommand)

## Success Criteria

1. ✓ Two agents can negotiate a contract via modal network
2. ✓ Formulas are stored and validated on-chain
3. ✓ Actions are rejected if they violate any formula
4. ✓ Signatures are verified via ed25519
5. ✓ Contract state is persisted and synced across nodes
