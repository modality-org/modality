# DAG Persistence Guide

This document explains how DAG (Directed Acyclic Graph) persistence works in the Shoal consensus implementation for modal-sequencer.

## Overview

The persistence layer provides:
- **Real-time certificate saves** - Every certificate is persisted immediately when processed
- **Checkpoint snapshots** - Full DAG state snapshots every 100 rounds for fast recovery
- **Flexible recovery** - Three recovery strategies (FromScratch, FromCheckpoint, Hybrid)
- **Automatic commit tracking** - Certificates are marked as committed in the datastore
- **Batch persistence** - Transaction batches are stored with certificate references

## Architecture

### Storage Models

The persistence layer uses four main models in `modal-datastore`:

#### 1. Certificate Model
Stores individual certificates with complete metadata:

```rust
pub struct Certificate {
    pub digest: String,              // Hex-encoded certificate digest (PK part)
    pub author: String,              // PeerId as base58 string
    pub round: u64,                  // Round number (PK part)
    pub header: String,              // JSON-serialized Header
    pub aggregated_signature: String,// JSON-serialized AggregatedSignature
    pub signers: Vec<bool>,          // Bitvec of signers
    pub batch_digest: String,        // Hex-encoded batch digest
    pub parents: Vec<String>,        // Parent certificate digests
    pub timestamp: u64,
    pub committed: bool,             // Marked when certificate is committed
    pub committed_at_round: Option<u64>,
    pub created_at: u64,
}
```

**Storage path**: `/dag/certificates/round/{round}/digest/{digest}`

#### 2. Batch Model
Stores transaction batches created by workers:

```rust
pub struct Batch {
    pub digest: String,              // Hex-encoded batch digest (PK)
    pub worker_id: u32,
    pub author: String,              // PeerId of validator
    pub transactions: String,        // JSON-serialized Vec<Transaction>
    pub transaction_count: usize,
    pub timestamp: u64,
    pub size_bytes: usize,
    pub referenced_by_cert: Option<String>, // Certificate that references this
    pub created_at: u64,
}
```

**Storage path**: `/dag/batches/digest/{digest}`

#### 3. DAGState Model
Checkpoint snapshots for fast recovery:

```rust
pub struct DAGState {
    pub checkpoint_round: u64,       // Primary key
    pub checkpoint_id: String,       // UUID
    pub highest_round: u64,
    pub certificate_count: usize,
    pub committed_count: usize,
    pub dag_snapshot: String,        // Base64-encoded bincode DAG
    pub consensus_state: String,     // JSON ConsensusState
    pub reputation_state: String,    // JSON ReputationState
    pub created_at: u64,
    pub size_bytes: usize,
}
```

**Storage path**: `/dag/checkpoints/round/{round}`

#### 4. ConsensusMetadata Model
Tracks overall consensus progress:

```rust
pub struct ConsensusMetadata {
    pub id: String,                  // Always "current" (singleton)
    pub current_round: u64,
    pub highest_committed_round: u64,
    pub last_anchor_round: Option<u64>,
    pub validator_peer_id: String,
    pub committee_size: usize,
    pub committee_epoch: u64,
    pub total_certificates: usize,
    pub total_committed: usize,
    pub total_batches: usize,
    pub total_transactions: u64,
    pub started_at: u64,
    pub last_updated: u64,
    pub last_checkpoint_at: u64,
}
```

**Storage path**: `/dag/metadata/id/current`

## Usage

### Enabling Persistence

Persistence is controlled by the `persistence` feature flag (enabled by default):

```toml
[dependencies]
modal-sequencer-consensus = { path = "../modal-sequencer-consensus", features = ["persistence"] }
```

### Initialization with Recovery

The `ShoalSequencer` automatically attempts DAG recovery on startup:

```rust
use modal_sequencer::shoal_sequencer::{ShoalSequencer, ShoalSequencerConfig};
use modal_datastore::NetworkDatastore;

let datastore = Arc::new(Mutex::new(NetworkDatastore::new(path)?));
let config = ShoalSequencerConfig::new(...);

// Sequencer will automatically try to recover DAG from datastore
let sequencer = ShoalSequencer::new(datastore, config).await?;
```

**Recovery process:**
1. Attempts hybrid recovery (checkpoint + incremental)
2. Falls back to full rebuild if checkpoint recovery fails
3. Starts fresh if no persisted data exists

### Real-time Certificate Persistence

Certificates are automatically persisted when processed:

```rust
// Certificate is persisted before processing
sequencer.process_certificate(cert).await?;
```

The persistence happens in `ShoalSequencer::process_certificate()`:
- Certificate is saved to datastore immediately
- If persistence fails, a warning is logged but processing continues
- This ensures no data loss even if the node crashes

### Automatic Checkpointing

Checkpoints are created automatically every 100 rounds:

```rust
// Checkpoint created at rounds 100, 200, 300, etc.
// Includes:
// - Complete DAG structure (bincode serialized)
// - Consensus state (committed certificates, anchors)
// - Reputation state (validator scores, performance records)
```

### Manual Persistence Operations

You can also perform manual persistence operations:

```rust
use modal_sequencer_consensus::persistence::recovery::{recover_dag, RecoveryStrategy};

// Manual recovery
let result = recover_dag(&datastore, RecoveryStrategy::FromScratch).await?;
println!("Loaded {} certificates", result.certificates_loaded);

// Manual checkpoint
let dag = dag_lock.read().await;
dag.create_checkpoint(round, &consensus_state, &reputation_state, &datastore).await?;

// Manual certificate persistence
dag.persist_certificate(&cert, &datastore).await?;

// Manual batch persistence
dag.persist_batch(&batch, &author_peer_id, Some(&cert_digest), &datastore).await?;
```

## Recovery Strategies

### FromScratch
Loads all certificates from datastore and rebuilds DAG:
- Most robust - always works if any certificates exist
- Slower for large DAGs (must load and process all certificates)
- Verifies DAG consistency during rebuild

```rust
let result = recover_dag(&datastore, RecoveryStrategy::FromScratch).await?;
```

### FromCheckpoint
Loads latest checkpoint and incremental certificates:
- Fastest recovery for large DAGs
- Requires valid checkpoint to exist
- Loads checkpoint + certificates since checkpoint

```rust
let result = recover_dag(&datastore, RecoveryStrategy::FromCheckpoint).await?;
```

### Hybrid (Recommended)
Tries checkpoint first, falls back to full rebuild:
- Best of both worlds
- Fast when checkpoints available
- Always succeeds if any data exists

```rust
let result = recover_dag(&datastore, RecoveryStrategy::Hybrid).await?;
```

## Query Operations

### Find Certificates

```rust
use modal_datastore::models::DAGCertificate;
use modal_datastore::Model;

// Find all certificates in a round
let certs = DAGCertificate::find_all_in_round(&datastore, 42).await?;

// Find certificates by author
let certs = DAGCertificate::find_by_author(&datastore, &peer_id.to_base58()).await?;

// Find all committed certificates
let committed = DAGCertificate::find_all_committed(&datastore).await?;

// Mark certificate as committed
cert_model.mark_committed(&datastore, commit_round).await?;
```

### Find Batches

```rust
use modal_datastore::models::DAGBatch;

// Find batches by author
let batches = DAGBatch::find_by_author(&datastore, &peer_id.to_base58()).await?;

// Find unreferenced batches
let unreferenced = DAGBatch::find_unreferenced(&datastore).await?;
```

### Checkpoint Management

```rust
use modal_datastore::models::DAGState;

// Get latest checkpoint
let checkpoint = DAGState::get_latest(&datastore).await?;

// Prune old checkpoints (keep only last 5)
DAGState::prune_old(&datastore, 5).await?;
```

### Consensus Metadata

```rust
use modal_datastore::models::ConsensusMetadata;

// Get current metadata
let mut metadata = ConsensusMetadata::get_current(&datastore).await?;

// Update and save
metadata.current_round = 100;
metadata.total_certificates += 1;
metadata.save(&datastore).await?;
```

## Performance Characteristics

### Storage Space

- **Certificate**: ~1-2 KB per certificate (depends on signers, parents)
- **Batch**: Variable (depends on transaction count and sizes)
- **Checkpoint**: 10-100 KB for small DAGs, can be MBs for large DAGs
- **Metadata**: <1 KB

### Recovery Times (approximate)

| Strategy | 1K certs | 10K certs | 100K certs |
|----------|----------|-----------|------------|
| FromScratch | <1s | 5-10s | 60-120s |
| FromCheckpoint | <100ms | <500ms | <2s |
| Hybrid | <100ms | <500ms | <2s |

### Write Performance

- **Certificate save**: <1ms (async, non-blocking)
- **Checkpoint creation**: 10-100ms (depending on DAG size)
- **Batch save**: <1ms

## Best Practices

### 1. Enable Persistence in Production
Always run with persistence enabled in production to ensure crash recovery.

### 2. Monitor Checkpoint Health
Periodically verify checkpoints are being created:
```rust
let checkpoint = DAGState::get_latest(&datastore).await?;
if checkpoint.is_none() || checkpoint.unwrap().checkpoint_round < current_round - 200 {
    log::warn!("Checkpoints are outdated!");
}
```

### 3. Prune Old Checkpoints
Regularly prune old checkpoints to save space:
```rust
// Keep only last 10 checkpoints
DAGState::prune_old(&datastore, 10).await?;
```

### 4. Handle Persistence Failures Gracefully
Persistence failures are logged but don't stop consensus:
```rust
if let Err(e) = dag.persist_certificate(&cert, &datastore).await {
    log::warn!("Persistence failed: {}", e);
    // Consensus continues regardless
}
```

### 5. Test Recovery Regularly
Periodically test recovery in development:
```rust
#[tokio::test]
async fn test_crash_recovery() {
    // Run consensus
    // Force crash
    // Restart and verify recovery
}
```

## Troubleshooting

### Recovery Fails with "no checkpoint found"
- **Cause**: No checkpoints exist yet
- **Solution**: Recovery will fall back to FromScratch automatically with Hybrid strategy

### Large Checkpoint Files
- **Cause**: DAG has grown very large
- **Solution**: 
  - Prune old checkpoints more aggressively
  - Consider implementing DAG pruning for old committed certificates

### Slow Recovery
- **Cause**: Loading many certificates from scratch
- **Solution**:
  - Ensure checkpoints are being created (every 100 rounds)
  - Use FromCheckpoint or Hybrid strategy
  - Verify checkpoint files aren't corrupted

### Persistence Errors During Consensus
- **Cause**: Datastore issues (disk full, permissions, etc.)
- **Solution**:
  - Check disk space
  - Verify datastore directory permissions
  - Review logs for specific error messages
  - Consensus will continue even if persistence fails

## Implementation Details

### Type Conversions

The persistence layer provides traits for converting between consensus types and storage models:

```rust
use modal_sequencer_consensus::persistence::{ToPersistenceModel, FromPersistenceModel};

// Convert consensus Certificate to storage model
let model = cert.to_persistence_model()?;
model.save(&datastore).await?;

// Convert storage model back to consensus Certificate
let cert = Certificate::from_persistence_model(&model)?;
```

### PeerId Handling

PeerIds are stored as base58 strings:
```rust
// To storage
let author_str = peer_id.to_base58();

// From storage
let peer_id = PeerId::from_str(&author_str)?;
```

### Digest Encoding

Digests are stored as hex strings:
```rust
// To storage
let digest_hex = hex::encode(digest);

// From storage  
let digest = hex::decode(&digest_hex)?;
let mut array = [0u8; 32];
array.copy_from_slice(&digest);
```

## Future Enhancements

Potential improvements to the persistence layer:

1. **DAG Pruning**: Remove very old committed certificates
2. **Incremental Checkpoints**: Only save delta since last checkpoint
3. **Compression**: Compress checkpoint data to reduce size
4. **Async Persistence**: Offload persistence to background task
5. **Replication**: Support for replicated datastores
6. **Metrics**: Add detailed persistence metrics and monitoring

## See Also

- [SHOAL_SPECIFICATION.md](./SHOAL_SPECIFICATION.md) - Shoal consensus protocol details
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Overall system architecture
- [Modal Datastore Documentation](../../modal-datastore/README.md) - Storage layer details

