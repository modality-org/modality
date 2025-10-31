# Shoal Consensus Architecture for Modal-Sequencer

## Overview

This document details the architecture for integrating Shoal consensus into modal-sequencer. It covers the transition from the current observer-based pattern to a full consensus implementation using Narwhal DAG and Shoal protocol.

## Current Architecture

### Existing Components

**modal-sequencer** (`rust/modal-sequencer/`)
```
src/
├── lib.rs              - Package exports
├── error.rs            - Error types
└── sequencer.rs        - Sequencer struct wrapping ChainObserver
```

Current functionality:
- Wraps `modal-observer::ChainObserver`
- Observes mining chain without participating
- Provides `get_chain_tip()`, `initialize()` APIs
- Uses `NetworkDatastore` for persistence

**modal-sequencer-consensus** (`rust/modal-sequencer-consensus/`)
```
src/
├── lib.rs              - Module exports
├── consensus_math.rs   - Quorum calculations (2f+1)
├── communication/      - Network communication abstractions
│   ├── mod.rs
│   └── same_process.rs - Single-process communication for testing
├── election/           - Leader election strategies
│   ├── mod.rs
│   ├── round_robin.rs
│   └── static_choice.rs
├── sequencing/         - Authority management
│   ├── mod.rs
│   └── static_authority.rs
└── runner.rs           - Consensus runner
```

Current functionality:
- Infrastructure for consensus protocols
- Quorum threshold calculation
- Communication abstractions
- Basic leader election (round-robin, static)
- Authority management trait

### Current Data Flow

```
User/Application
        ↓
  Sequencer
        ↓
  ChainObserver (modal-observer)
        ↓
  NetworkDatastore
```

The sequencer passively observes blocks mined by others and tracks the canonical chain.

## Target Architecture

### High-Level Design

```
User/Application
        ↓
┌─────────────────────────────────────┐
│  Sequencer (modal-sequencer)        │
│  - Transaction submission           │
│  - State machine execution          │
│  - Query interface                  │
└─────────────────────────────────────┘
        ↓
┌─────────────────────────────────────┐
│  Shoal Consensus                    │
│  (modal-sequencer-consensus)        │
│                                     │
│  ┌─────────────────────────────┐  │
│  │  Shoal Layer                │  │
│  │  - Anchor selection         │  │
│  │  - Leader reputation        │  │
│  │  - Commit rules             │  │
│  │  - Transaction ordering     │  │
│  └─────────────────────────────┘  │
│              ↓                      │
│  ┌─────────────────────────────┐  │
│  │  Narwhal Layer              │  │
│  │  - Certificate formation    │  │
│  │  - DAG construction         │  │
│  │  - Batch availability       │  │
│  │  - Primary/Worker nodes     │  │
│  └─────────────────────────────┘  │
└─────────────────────────────────────┘
        ↓
┌─────────────────────────────────────┐
│  Network Communication              │
│  - Gossip protocol                  │
│  - Vote collection                  │
│  - Batch requests                   │
└─────────────────────────────────────┘
        ↓
┌─────────────────────────────────────┐
│  NetworkDatastore                   │
│  - DAG persistence                  │
│  - Consensus state                  │
│  - Reputation data                  │
└─────────────────────────────────────┘
```

### Component Breakdown

## Narwhal Layer Components

### Directory Structure

```
rust/modal-sequencer-consensus/src/narwhal/
├── mod.rs              - Module exports
├── types.rs            - Core data structures (Batch, Header, Certificate)
├── dag.rs              - DAG storage and management
├── certificate.rs      - Certificate formation and verification
├── worker.rs           - Worker node implementation
└── primary.rs          - Primary node implementation
```

### Core Types

**Batch** (`narwhal/types.rs`)
```rust
pub struct Batch {
    pub transactions: Vec<Transaction>,
    pub worker_id: WorkerId,
    pub timestamp: u64,
}

pub type Digest = [u8; 32];  // SHA-256
pub type BatchDigest = Digest;
```

**Header** (`narwhal/types.rs`)
```rust
pub struct Header {
    pub author: PublicKey,
    pub round: u64,
    pub batch_digest: BatchDigest,
    pub parents: Vec<CertificateDigest>,
    pub timestamp: u64,
}

impl Header {
    pub fn digest(&self) -> Digest;
    pub fn verify_parents(&self, dag: &DAG, expected_round: u64) -> Result<()>;
}
```

**Certificate** (`narwhal/types.rs`)
```rust
pub struct Certificate {
    pub header: Header,
    pub aggregated_signature: AggregatedSignature,
    pub signers: BitVec,
}

pub type CertificateDigest = Digest;

impl Certificate {
    pub fn verify(&self, committee: &Committee) -> Result<()>;
    pub fn has_quorum(&self, total_validators: usize) -> bool;
    pub fn digest(&self) -> CertificateDigest;
}
```

**Committee** (validator set)
```rust
pub struct Committee {
    pub validators: HashMap<PublicKey, Validator>,
    pub total_stake: u64,
}

pub struct Validator {
    pub public_key: PublicKey,
    pub stake: u64,
    pub network_address: SocketAddr,
}

impl Committee {
    pub fn quorum_threshold(&self) -> u64 {
        consensus_math::calculate_2f_plus_1(self.validators.len() as f64)
    }
}
```

### DAG Implementation

**DAG Storage** (`narwhal/dag.rs`)
```rust
pub struct DAG {
    // Primary storage: digest -> certificate
    certificates: HashMap<CertificateDigest, Certificate>,
    
    // Index by round for efficient queries
    by_round: BTreeMap<u64, Vec<CertificateDigest>>,
    
    // Index by author
    by_author: HashMap<PublicKey, BTreeMap<u64, CertificateDigest>>,
    
    // Persistent storage
    datastore: Arc<Mutex<NetworkDatastore>>,
}

impl DAG {
    pub async fn insert(&mut self, cert: Certificate) -> Result<()>;
    pub fn get(&self, digest: &CertificateDigest) -> Option<&Certificate>;
    pub fn get_round(&self, round: u64) -> Vec<&Certificate>;
    pub fn get_author_cert(&self, author: &PublicKey, round: u64) -> Option<&Certificate>;
    pub fn has_path(&self, from: &CertificateDigest, to: &CertificateDigest) -> bool;
    pub fn detect_equivocation(&self, cert: &Certificate) -> bool;
    pub async fn persist(&self) -> Result<()>;
    pub async fn load(&mut self) -> Result<()>;
}
```

**Certificate Formation** (`narwhal/certificate.rs`)
```rust
pub struct CertificateBuilder {
    header: Header,
    votes: HashMap<PublicKey, Signature>,
    committee: Committee,
}

impl CertificateBuilder {
    pub fn new(header: Header, committee: Committee) -> Self;
    pub fn add_vote(&mut self, voter: PublicKey, signature: Signature) -> Result<()>;
    pub fn has_quorum(&self) -> bool;
    pub fn build(self) -> Result<Certificate>;
}
```

### Worker/Primary Architecture

**Worker** (`narwhal/worker.rs`)
```rust
pub struct Worker {
    id: WorkerId,
    validator: PublicKey,
    batch_size: usize,
    max_batch_bytes: usize,
    tx_buffer: Vec<Transaction>,
    storage: Arc<Mutex<HashMap<BatchDigest, Batch>>>,
}

impl Worker {
    pub async fn collect_transactions(&mut self) -> Result<()>;
    pub async fn form_batch(&mut self) -> Result<(Batch, BatchDigest)>;
    pub async fn serve_batch(&self, digest: BatchDigest) -> Result<Batch>;
}
```

**Primary** (`narwhal/primary.rs`)
```rust
pub struct Primary {
    validator: PublicKey,
    keypair: Keypair,
    committee: Committee,
    dag: Arc<RwLock<DAG>>,
    current_round: AtomicU64,
    workers: Vec<Arc<Worker>>,
    network: Arc<dyn Communication>,
}

impl Primary {
    pub async fn propose(&mut self) -> Result<CertificateDigest>;
    pub async fn process_header(&self, header: Header) -> Result<()>;
    pub async fn process_vote(&mut self, vote: Vote) -> Result<Option<Certificate>>;
    pub async fn advance_round(&mut self);
}
```

## Shoal Consensus Layer

### Directory Structure

```
rust/modal-sequencer-consensus/src/shoal/
├── mod.rs              - Module exports
├── types.rs            - Consensus data structures
├── reputation.rs       - Leader reputation system
├── consensus.rs        - Pipelined consensus logic
└── ordering.rs         - Certificate ordering
```

### Core Types

**Reputation State** (`shoal/types.rs`)
```rust
pub struct ReputationState {
    scores: HashMap<PublicKey, f64>,
    recent_performance: VecDeque<PerformanceRecord>,
    config: ReputationConfig,
}

pub struct PerformanceRecord {
    pub validator: PublicKey,
    pub round: u64,
    pub latency_ms: u64,
    pub success: bool,
    pub timestamp: u64,
}

pub struct ReputationConfig {
    pub window_size: usize,
    pub decay_factor: f64,
    pub min_score: f64,
    pub target_latency_ms: u64,
}
```

**Consensus State** (`shoal/types.rs`)
```rust
pub struct ConsensusState {
    pub current_round: u64,
    pub anchors: BTreeMap<u64, CertificateDigest>,
    pub committed: BTreeSet<CertificateDigest>,
    pub last_committed_round: u64,
}
```

### Reputation System

**Reputation Manager** (`shoal/reputation.rs`)
```rust
pub struct ReputationManager {
    state: ReputationState,
    committee: Committee,
}

impl ReputationManager {
    pub fn new(committee: Committee, config: ReputationConfig) -> Self;
    
    pub fn select_leader(&self, round: u64) -> PublicKey;
    
    pub fn record_performance(&mut self, record: PerformanceRecord);
    
    pub fn update_scores(&mut self);
    
    pub fn get_score(&self, validator: &PublicKey) -> f64;
    
    fn calculate_round_performance(&self, latency_ms: u64, success: bool) -> f64;
}
```

### Consensus Engine

**Shoal Consensus** (`shoal/consensus.rs`)
```rust
pub struct ShoalConsensus {
    dag: Arc<RwLock<DAG>>,
    reputation: ReputationManager,
    state: ConsensusState,
    committee: Committee,
}

impl ShoalConsensus {
    pub async fn process_certificate(&mut self, cert: Certificate) -> Result<Vec<CertificateDigest>>;
    
    pub async fn select_anchor(&self, round: u64) -> Result<CertificateDigest>;
    
    pub async fn check_commit_rule(&self, anchor: &CertificateDigest) -> Result<bool>;
    
    pub async fn commit_certificate(&mut self, digest: CertificateDigest) -> Result<Vec<CertificateDigest>>;
    
    pub async fn advance_round(&mut self);
}
```

**Commit Rule Implementation**
```rust
impl ShoalConsensus {
    async fn check_commit_rule(&self, anchor: &CertificateDigest) -> Result<bool> {
        let dag = self.dag.read().await;
        let anchor_cert = dag.get(anchor).ok_or(Error::CertificateNotFound)?;
        let prev_round = anchor_cert.header.round.saturating_sub(1);
        
        // Get anchors from previous round
        let prev_anchors = self.state.anchors
            .range(prev_round..prev_round+1)
            .map(|(_, digest)| digest)
            .collect::<Vec<_>>();
        
        // Need path to at least 2f+1 anchors from previous round
        let quorum = self.committee.quorum_threshold();
        let mut reachable_count = 0;
        
        for prev_anchor in prev_anchors {
            if dag.has_path(anchor, prev_anchor) {
                reachable_count += 1;
            }
        }
        
        Ok(reachable_count >= quorum as usize)
    }
}
```

### Certificate Ordering

**Ordering Engine** (`shoal/ordering.rs`)
```rust
pub struct OrderingEngine {
    dag: Arc<RwLock<DAG>>,
}

impl OrderingEngine {
    pub async fn order_certificates(
        &self,
        committed: &BTreeSet<CertificateDigest>
    ) -> Result<Vec<Transaction>> {
        let dag = self.dag.read().await;
        
        // 1. Topological sort respecting DAG edges
        let ordered_certs = self.topological_sort(&dag, committed)?;
        
        // 2. Extract transactions from ordered certificates
        let mut transactions = Vec::new();
        for cert_digest in ordered_certs {
            let cert = dag.get(&cert_digest).unwrap();
            let batch = self.fetch_batch(&cert.header.batch_digest).await?;
            transactions.extend(batch.transactions);
        }
        
        Ok(transactions)
    }
    
    fn topological_sort(
        &self,
        dag: &DAG,
        certs: &BTreeSet<CertificateDigest>
    ) -> Result<Vec<CertificateDigest>> {
        // Kahn's algorithm with deterministic tie-breaking
        // Tie-break by: (round, author_id)
        // Implementation details...
    }
}
```

## Integration with Modal-Sequencer

### Updated Sequencer

**Sequencer** (`modal-sequencer/src/sequencer.rs`)
```rust
pub struct Sequencer {
    config: SequencerConfig,
    primary: Primary,
    consensus: ShoalConsensus,
    datastore: Arc<Mutex<NetworkDatastore>>,
}

pub struct SequencerConfig {
    pub validator_keypair: Keypair,
    pub committee: Committee,
    pub narwhal_config: NarwhalConfig,
    pub shoal_config: ShoalConfig,
}

impl Sequencer {
    pub async fn new(
        datastore: Arc<Mutex<NetworkDatastore>>,
        config: SequencerConfig,
    ) -> Result<Self>;
    
    pub async fn initialize(&self) -> Result<()>;
    
    // New: Transaction submission
    pub async fn submit_transaction(&self, tx: Transaction) -> Result<()>;
    
    // New: Get ordered transactions
    pub async fn get_committed_transactions(&self, from: u64, to: u64) -> Result<Vec<Transaction>>;
    
    // Updated: Chain tip now means last committed round
    pub async fn get_chain_tip(&self) -> u64;
    
    // Internal: Run consensus
    async fn run_consensus_loop(&mut self);
}
```

### Networking Integration

**Communication Trait** (extend existing)
```rust
#[async_trait]
pub trait Communication: Send + Sync {
    // Existing methods...
    
    // New: Narwhal messages
    async fn broadcast_header(&self, header: Header) -> Result<()>;
    async fn send_vote(&self, to: PublicKey, vote: Vote) -> Result<()>;
    async fn broadcast_certificate(&self, cert: Certificate) -> Result<()>;
    async fn request_batch(&self, from: PublicKey, digest: BatchDigest) -> Result<Batch>;
    
    // New: Message handlers
    async fn on_header_received(&self, header: Header);
    async fn on_vote_received(&self, vote: Vote);
    async fn on_certificate_received(&self, cert: Certificate);
    async fn on_batch_request(&self, request: BatchRequest);
}
```

**Network Protocol** (new implementation)
```rust
pub struct GossipNetwork {
    local_addr: SocketAddr,
    peers: HashMap<PublicKey, SocketAddr>,
    message_handlers: Arc<MessageHandlers>,
}

impl Communication for GossipNetwork {
    // Implementation using tokio networking
    // Gossip protocol for efficient broadcast
    // Request/response for batch availability
}
```

### Persistence Integration

**DAG Storage** (using NetworkDatastore)
```rust
// Store certificates
key: format!("cert:{}", cert_digest)
value: bincode::serialize(&certificate)

// Store DAG indices
key: format!("round:{}:certs", round)
value: bincode::serialize(&vec_of_cert_digests)

// Store consensus state
key: "consensus:state"
value: bincode::serialize(&consensus_state)

// Store reputation state
key: "consensus:reputation"
value: bincode::serialize(&reputation_state)

// Store batches
key: format!("batch:{}", batch_digest)
value: bincode::serialize(&batch)
```

## Data Flow

### Transaction Submission Flow

```
1. User submits transaction
   ↓
2. Sequencer.submit_transaction()
   ↓
3. Worker.collect_transactions()
   ↓
4. Worker.form_batch()
   ↓
5. Primary.propose() [creates header]
   ↓
6. Network.broadcast_header()
   ↓
7. Other validators vote
   ↓
8. Primary.process_vote() [collects 2f+1]
   ↓
9. Primary forms Certificate
   ↓
10. Network.broadcast_certificate()
    ↓
11. DAG.insert(certificate)
    ↓
12. ShoalConsensus.process_certificate()
    ↓
13. ShoalConsensus.check_commit_rule()
    ↓
14. OrderingEngine.order_certificates()
    ↓
15. Sequencer executes ordered transactions
```

### Round Progression

```
Round N:
├── Primary proposes header with batch
├── Validators vote (2f+1 collected)
├── Certificate formed and broadcast
└── Added to DAG

Consensus (parallel):
├── Reputation selects leader for round N
├── Leader's certificate becomes anchor
├── Check commit rule (path to 2f+1 prev anchors)
├── If satisfied: commit certificate
└── Update reputation based on performance

Round N+1:
├── (Same as Round N, pipelined)
└── ...
```

## Migration Strategy

### Phase 1: Parallel Implementation
- Keep existing observer-based sequencer
- Implement Narwhal + Shoal in separate modules
- Test independently

### Phase 2: Integration Testing
- Wire up components
- Run in test mode alongside observer
- Compare outputs for consistency

### Phase 3: Gradual Rollout
- Deploy to testnet
- Monitor performance and stability
- Iterate based on real-world data

### Phase 4: Production Deployment
- Replace observer-based approach
- Full Shoal consensus in production

## Testing Strategy

### Unit Tests
- Certificate verification
- DAG insertion/queries
- Reputation score calculation
- Commit rule validation
- Ordering determinism

### Integration Tests
- Multi-validator scenarios (3, 4, 7, 10 validators)
- Byzantine behavior (equivocation, withholding)
- Network partitions
- Crash recovery

### Performance Tests
- Throughput benchmarks (target: 125K TPS)
- Latency measurements (target: ~1.2s)
- Resource usage (CPU, memory, network)
- Scalability testing (increasing validators)

## Monitoring and Observability

### Metrics to Track
- Throughput (TPS)
- Latency (p50, p95, p99)
- Certificate formation time
- Round progression rate
- Reputation scores per validator
- DAG size and growth rate
- Network message rates

### Logging
- Certificate events (proposed, voted, formed)
- Consensus decisions (anchor selection, commits)
- Reputation updates
- Errors and anomalies

## Security Considerations

### Cryptography
- Ed25519 for signatures (fast, secure)
- SHA-256 for digests
- BLS for signature aggregation (future optimization)

### Attack Mitigation
- Equivocation detection and slashing
- Rate limiting on message processing
- DoS protection (reject invalid messages early)
- Reputation system handles slow/malicious validators

### Operational Security
- Key management (secure storage)
- Network authentication (TLS)
- Access control (validator set management)

---

**Document Status**: Architecture Design v1.0  
**Last Updated**: 2025-10-30  
**Target Implementation**: modal-sequencer  

