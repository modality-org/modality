# Modality: A Hybrid Blockchain for Verifiable Contracts

## Abstract

Modality is a novel blockchain platform that combines permissionless proof-of-work mining with fast-finality BFT consensus to enable verifiable smart contracts using temporal modal logic and WASM-based programmability. By integrating RandomX PoW for open participation, Shoal BFT for rapid confirmation, and a formal verification system based on labeled transition systems, Modality addresses key limitations in existing blockchains: slow finality in PoW systems, permissioned entry in BFT systems, and lack of built-in verification for contracts. This whitepaper presents the system's architecture, key protocols, security analysis, and performance considerations, demonstrating how Modality achieves decentralized, secure, and verifiable computation.

## Introduction

Blockchain technology has revolutionized decentralized systems, but existing platforms suffer from trade-offs between decentralization, finality speed, and contract verifiability. Proof-of-work (PoW) chains like Bitcoin offer permissionless entry but probabilistic finality, while Byzantine Fault Tolerant (BFT) systems like Tendermint provide fast finality but require permissioned validator sets. Moreover, smart contracts often lack formal verification, leading to vulnerabilities.

Modality introduces a hybrid architecture that leverages PoW for permissionless validator nomination and BFT for fast consensus, coupled with a native modal logic system for verifiable contracts. Key innovations include:

- **Permissionless Entry**: Anyone can mine to nominate validators using CPU-friendly RandomX PoW.
- **Fast Finality**: Shoal BFT achieves consensus in seconds on mined blocks.
- **Programmable Contracts**: WASM modules enable dynamic logic with gas metering.
- **Modal Verification**: Rule-based temporal logic ensures contract compliance.

This paper is structured as follows: Section 2 provides background; Section 3 overviews the architecture; Sections 4-7 detail core components; Section 8 covers integration; Sections 9-10 analyze security and performance; Section 11 discusses future work; and Section 12 concludes.

## Background & Motivation

### Blockchain Consensus Trade-offs

PoW systems like Bitcoin [1] enable open participation but suffer from slow finality (hours for high confidence) and vulnerability to 51% attacks. BFT systems like PBFT [2] offer deterministic finality but require known validator sets, limiting decentralization.

Building on foundational work in digital timestamping [3], which introduced mechanisms for creating tamper-evident chains of documents, modern blockchains extend these ideas to achieve consensus in distributed systems.

Hybrid approaches like Ethereum 2.0 [4] use PoS for finality on PoW-mined blocks, but PoS introduces economic barriers. Modality's hybrid PoW/BFT design allows permissionless entry without staking requirements.

### Formal Verification in Blockchains

Smart contract bugs have caused losses exceeding $1B [5]. While tools like Solidity verifiers exist, they are not native to blockchains. Temporal logics like LTL [6] are effective for state machines, but rarely integrated.

Modality embeds modal logic directly, allowing contracts to specify and verify temporal properties natively.

### Motivation

Modality motivates a blockchain where:
- Entry is truly permissionless (PoW mining).
- Transactions finalize quickly (BFT).
- Contracts are programmatically verifiable (WASM + modal logic).

Use cases include decentralized finance with verified rules, supply chain tracking with temporal guarantees, and AI agent coordination with formal constraints.

## System Architecture

Modality's architecture comprises two layers: a PoW mining layer for block production and validator nomination, and a BFT consensus layer for finality. Nodes run `modal-node`, integrating `modal-miner` for PoW, `modal-validator-consensus` for BFT, `modal-wasm-runtime` for contracts, and `modality-lang` for verification.

```
+-------------------+     +-------------------+
| PoW Mining Layer  |     | BFT Consensus Layer|
| - RandomX PoW     |<--->| - Shoal Algorithm  |
| - Block Production|     | - DAG Processing   |
| - Nominations     |     | - Reputation Mgmt  |
+-------------------+     +-------------------+
          |                           |
          v                           v
+-------------------+     +-------------------+
| Contract Execution |     | Verification Layer |
| - WASM Runtime    |<--->| - Modal Logic      |
| - Gas Metering    |     | - Model Checking   |
+-------------------+     +-------------------+
```

Key flows:
- Miners produce blocks, nominating validators.
- Validators form sets from nominees and run BFT on blocks.
- Contracts execute WASM with modal verification.

Add detailed component descriptions:

The PoW layer uses modal-miner for block production. Key files: rust/modal-miner/src/block.rs defines Block and BlockHeader; rust/modal-miner/src/miner.rs handles nonce finding with RandomX.

The BFT layer uses modal-validator-consensus. Key: rust/modal-validator-consensus/src/shoal/consensus.rs for certificate processing and anchor selection.

Contract layer: modal-wasm-runtime/src/executor.rs for gas-metered execution.

Verification: modality-lang/src/model_checker.rs for formula evaluation.

## Permissionless Entry via Proof-of-Work

Modality uses RandomX [7] for ASIC-resistant, CPU-friendly mining, ensuring broad participation.

### Block Structure

From `rust/modal-miner/src/block.rs`:

```
pub struct BlockData {
    pub nominated_peer_id: String,
    pub miner_number: u64,
}

pub struct BlockHeader {
    pub index: u64,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: String,
    pub data_hash: String,
    pub nonce: u128,
    pub difficulty: u128,
    pub hash: String,
}
```

Blocks include a nominated peer ID for validator selection.

Expand: The hash is computed using RandomX in header.calculate_hash, ensuring ASIC resistance.

### Mining Algorithm

Miners solve for nonce such that `hash(mining_data + nonce) < difficulty_target`. Using `hash_tax` for RandomX.

From `rust/modal-miner/src/miner.rs`:

```
pub fn mine_block_with_stats(&self, block: Block) -> Result<MinedBlockResult> {
    let mining_result = hash_tax::mine_with_stats(
        &mining_data,
        difficulty,
        self.config.max_tries,
        "randomx",
    )?;
    // ...
}
```

Add: Difficulty is u128, allowing fine-grained adjustments. Verification in verify_block checks nonce validity.

### Epoch-Based Adjustment

Epochs are 40 blocks. Difficulty adjusts to target block time.

From `rust/modal-miner/src/epoch.rs`:

```
fn adjust_difficulty(previous_epoch: &Epoch) -> u128 {
    let target_duration = EPOCH_DURATION_TARGET;
    let actual_duration = previous_epoch.end_time - previous_epoch.start_time;
    previous_epoch.difficulty * target_duration / actual_duration
}
```

Add: From epoch.rs, epochs track start/end times, nominated peers, and adjust based on actual vs target duration.

### Nomination and Shuffling

Each block nominates a peer ID. Epoch nominees are shuffled using Fisher-Yates with XOR seed of nonces, ensuring fairness.

Expand: Shuffling in epoch.rs uses Fisher-Yates with seed from XOR of block nonces, preventing bias.

## Fast Finality via Shoal BFT Consensus

Shoal is a reputation-based BFT protocol built on Narwhal DAG [8].

### Validator Selection

From `rust/modal-datastore/src/models/validator/validator_selection.rs`:

```
pub async fn generate_validator_set_from_epoch(datastore: &NetworkDatastore, epoch: u64) -> Result<ValidatorSet> {
    let seed = calculate_epoch_seed(&epoch_blocks);
    let mut nominees = shuffle_nominees(&epoch_blocks, seed);
    ValidatorSet::new(
        epoch,
        nominees.drain(0..27).collect(),
        get_top_staked(13), // Placeholder
        nominees.drain(0..13).collect(),
    )
}
```

Hybrid mode uses 2-epoch lookback for stability.

Expand: In validator_selection.rs, get_validator_set_for_epoch first checks static validators, falls back to dynamic from mining blocks. Hybrid mode uses nomination_epoch = current - 2.

### Shoal Algorithm

From `rust/modal-validator-consensus/src/shoal/consensus.rs`:

```
pub async fn process_certificate(&mut self, cert: Certificate) -> Result<Vec<CertificateDigest>> {
    // Add to DAG
    self.dag.write().await.insert(cert.clone())?;
    // Update reputation
    self.record_certificate_performance(&cert);
    // Select anchor if possible
    if let Some(anchor) = self.try_select_anchor(round).await? {
        if self.check_commit_rule(&anchor).await? {
            return self.commit_certificate(anchor).await;
        }
    }
}
```

Leaders are selected by reputation scores, tolerating f < n/3 faults.

Add: Reputation in reputation.rs selects leaders by score, with deterministic tie-breaking via hash(round || key).

Commit rule checks 2f+1 support in DAG.

## Programmable Contracts via WASM Modules

Contracts use WASM for dynamic logic.

### Runtime

From `rust/modal-wasm-runtime/src/executor.rs`:

```
pub fn execute(&mut self, wasm_bytes: &[u8], method: &str, args: &str) -> Result<String> {
    // Compile and instantiate module with fuel
    store.set_fuel(self.gas_limit)?;
    // Call exported method
    let result_ptr = method_func.call(&mut store, (args_ptr, args_len))?;
    // ...
}
```

Gas metering prevents DoS.

Add: Supports alloc for memory, basic host functions. Returns JSON with valid, gas_used, errors.

### Predicate System

Dynamic properties via WASM predicates, e.g., `signed_by`.

Expand: Standard predicates like signed_by verify signatures deterministically.

## Modal Contracts via Temporal Logic

Contracts as LTS with modal formulas.

From `rust/modality-lang/src/ast.rs`:

```
pub enum FormulaExpr {
    Diamond(Vec<Property>, Box<FormulaExpr>),
    Box(Vec<Property>, Box<FormulaExpr>),
    // ...
}
```

Model checker verifies satisfaction.

Add: Grammar in grammar.lalrpop defines syntax for models, formulas, properties.

Checker in model_checker.rs evaluates satisfaction per part or any-state.

## System Integration

Mining nominates validators; BFT finalizes blocks; WASM executes with modal verification.

Expand: In modal-node/src/actions/miner.rs, mine_and_gossip_block selects nominees rotationally.

Validator in actions/validator.rs processes via Shoal.

Sync in CHAIN_SYNC.md describes orphan rejection and active sync.

## Security Analysis

- PoW: Cumulative difficulty resists reorgs.
- BFT: Tolerates f < n/3.
- WASM: Sandboxed with gas limits.

Expand: Fork choice in FORK_CHOICE.md uses cumulative difficulty.

WASM security: no external access, fuel limits.

## Performance Considerations

- Mining: Adjustable difficulty.
- Consensus: Seconds finality.
- WASM: Cached execution, low gas for predicates.

Add: Benchmarks from benches/consensus_benchmarks.rs show throughput.

WASM cache in cache.rs provides LRU with max 100 modules.

## Future Work

Sharding, advanced staking, formal proofs.

## Conclusion

Modality uniquely combines permissionless entry, fast finality, and verifiable contracts.

References:
[1] Nakamoto, Bitcoin Whitepaper
[2] Castro & Liskov, PBFT
[3] Haber & Stornetta, How to time-stamp a digital document
[3] Buterin, Ethereum 2.0
[4] Various DeFi hack reports
[5] Pnueli, Temporal Logic
[6] RandomX Documentation
[7] Narwhal Paper
[8] Shoal Paper

