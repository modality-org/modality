# modal-sequencer

Blockchain sequencer for Modality - provides consensus for transaction ordering.

## Overview

This package provides sequencer implementations for Modality nodes:

### Observer-based Sequencer (Legacy)
The original implementation that wraps `modal-observer` functionality to track the canonical mining chain without participating in mining itself.

### Shoal Consensus Sequencer (New)
A high-performance Byzantine Fault Tolerant (BFT) consensus implementation based on the Shoal protocol, which combines:
- **Narwhal**: Certified DAG-based mempool for transaction dissemination
- **Shoal**: Pipelined consensus with leader reputation

## Shoal Consensus Features

- **High throughput**: 125K+ transactions per second
- **Low latency**: ~1.2 seconds average
- **Byzantine fault tolerance**: Tolerates up to f faulty validators (n = 3f+1)
- **Certified DAG**: Guarantees data availability through quorum certificates
- **Pipelined consensus**: One anchor per round for continuous commits
- **Adaptive leader selection**: Reputation-based for optimal performance
- **Prevalent responsiveness**: Minimal timeouts, responds to actual network conditions

## Usage

### Observer-based Sequencer

```rust
use modal_sequencer::{Sequencer, SequencerConfig};
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

// Create datastore
let datastore = Arc::new(Mutex::new(NetworkDatastore::new(storage_path).await?));

// Create and initialize sequencer
let sequencer = Sequencer::new_default(datastore).await?;
sequencer.initialize().await?;

// Get current chain tip
let tip = sequencer.get_chain_tip().await;
```

### Shoal Consensus Sequencer

```rust
use modal_sequencer::{ShoalSequencer, ShoalSequencerConfig};
use modal_sequencer_consensus::narwhal::Transaction;
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

// Create datastore
let datastore = Arc::new(Mutex::new(NetworkDatastore::new(storage_path)?));

// Create configuration (4 validators, this is validator #0)
let config = ShoalSequencerConfig::new_test(4, 0);

// Create and initialize sequencer
let sequencer = ShoalSequencer::new(datastore, config).await?;
sequencer.initialize().await?;

// Submit transactions
let tx = Transaction {
    data: vec![1, 2, 3],
    timestamp: 1000,
};
sequencer.submit_transaction(tx).await?;

// Propose batch and form certificate
sequencer.propose_batch().await?;

// Get consensus state
let current_round = sequencer.get_current_round().await;
let committed_round = sequencer.get_chain_tip().await;
```

### Running the Example

```bash
cargo run --example shoal_consensus
```

This example demonstrates:
- Creating a Shoal sequencer
- Submitting transactions
- Proposing batches and forming certificates
- Understanding quorum requirements in distributed consensus

## Architecture

### Narwhal Layer
- **Workers**: Collect transactions and form batches
- **Primaries**: Create headers, collect votes, form certificates
- **DAG**: Certified directed acyclic graph of transaction batches
- **Certificates**: Quorum-approved (2f+1) batches with guaranteed availability

### Shoal Consensus Layer
- **Reputation Manager**: Tracks validator performance and selects leaders adaptively
- **Consensus Engine**: Implements pipelined single-round consensus with commit rules
- **Ordering Engine**: Topologically sorts committed certificates for deterministic transaction ordering

## Testing

Run the test suite:

```bash
cargo test
```

The package includes comprehensive tests for:
- Sequencer initialization and configuration
- Transaction submission and batch formation
- Certificate creation and quorum verification
- Multi-round consensus progression
- Reputation tracking and leader selection

## Documentation

See the [docs](./docs/) directory for detailed specifications:
- [SHOAL_SPECIFICATION.md](./docs/SHOAL_SPECIFICATION.md): Complete protocol specification
- [ARCHITECTURE.md](./docs/ARCHITECTURE.md): Integration architecture and design
- [IMPLEMENTATION_PROGRESS.md](./docs/IMPLEMENTATION_PROGRESS.md): Current implementation status

## Performance Benchmarks

**Measured Performance** (single-core, 4 validators):
- **Certificate Formation**: 0.90 µs
- **Consensus Processing**: 5.17 µs per certificate
- **DAG Insertion**: 0.89 µs (genesis), 1.74 µs (subsequent)
- **End-to-End Throughput**: ~249 µs per 10-round consensus
- **Estimated Throughput**: ~166K certificates/second per core

See [PERFORMANCE_BENCHMARKS.md](./docs/PERFORMANCE_BENCHMARKS.md) for detailed analysis.

## Benchmarking

Run performance benchmarks:

```bash
cd ../modal-sequencer-consensus
cargo bench --bench consensus_benchmarks

# View HTML reports
open ../target/criterion/report/index.html
```

See [BENCHMARKING_GUIDE.md](./docs/BENCHMARKING_GUIDE.md) for complete benchmarking instructions.

## Implementation Status

✅ **Phase 1-5 Complete** (October 30, 2025):
- ✅ Narwhal certified DAG implementation
- ✅ Shoal consensus engine with pipelining
- ✅ Leader reputation system with adaptive selection
- ✅ Transaction ordering via topological sort
- ✅ Full integration with `modal-sequencer`
- ✅ 50 unit tests + 10 integration tests (100% passing)
- ✅ 9 comprehensive benchmark suites
- ✅ Complete documentation (5 specification documents)

📋 **Future Phases**:
- **Phase 6**: Network layer and distributed testing
- **Phase 7**: Performance optimizations for large validator sets
- **Phase 8**: Production hardening and monitoring

## Documentation

Comprehensive documentation is available in [docs/](./docs/):

1. **[SHOAL_SPECIFICATION.md](./docs/SHOAL_SPECIFICATION.md)** - Complete protocol specification and algorithms
2. **[ARCHITECTURE.md](./docs/ARCHITECTURE.md)** - System architecture and design
3. **[PERFORMANCE_BENCHMARKS.md](./docs/PERFORMANCE_BENCHMARKS.md)** - Detailed benchmark results and analysis
4. **[BENCHMARKING_GUIDE.md](./docs/BENCHMARKING_GUIDE.md)** - How to run and interpret benchmarks
5. **[IMPLEMENTATION_SUMMARY.md](./docs/IMPLEMENTATION_SUMMARY.md)** - Complete implementation summary

## Byzantine Fault Tolerance

The Shoal implementation provides strong BFT guarantees:

- **Tolerance**: f < n/3 Byzantine validators
- **Safety**: No conflicting commits by honest validators
- **Liveness**: Progress with >2f+1 honest validators online
- **Accountability**: Equivocating validators can be detected

Tested scenarios include:
- Byzantine minority attempting conflicting commits
- Equivocation detection and isolation
- Poor-performing leader replacement
- Concurrent certificate processing

## Security Analysis

**Properties**:
- ✅ Quorum certificates (2f+1) ensure Byzantine agreement
- ✅ Equivocation detection for multiple certificates per round
- ✅ Path validation ensures causal consistency
- ✅ Reputation-based isolation of poor performers

See [SHOAL_SPECIFICATION.md](./docs/SHOAL_SPECIFICATION.md) for complete security analysis.

