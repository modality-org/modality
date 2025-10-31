# Shoal Consensus Protocol Specification

## Overview

This document specifies the Shoal consensus protocol implementation for modal-sequencer. Shoal combines **Narwhal** (a certified DAG-based mempool) with **Shoal consensus** (a pipelined ordering protocol with leader reputation) to achieve high-throughput (125K+ TPS), low-latency (~1.2s) Byzantine Fault Tolerant consensus.

**Key Properties:**
- Byzantine fault tolerance: Tolerates up to f Byzantine validators where n = 3f + 1
- Certified DAG: Guarantees data availability through quorum certificates
- Pipelined consensus: One anchor per round for continuous commits
- Leader reputation: Adaptive leader selection based on performance
- Prevalent responsiveness: Minimal timeouts, responds to actual network conditions

## Part 1: Narwhal Certified DAG

Narwhal is a high-throughput mempool protocol that separates transaction dissemination from ordering. It constructs a certified Directed Acyclic Graph (DAG) where vertices are batches of transactions certified by quorum.

### 1.1 Architecture

**Validator Nodes:**
Each validator in the network has two types of nodes:
- **Primary**: Creates headers, collects votes, forms certificates, participates in consensus
- **Workers** (1+): Collect transactions, form batches, serve batch data to other validators

**Separation of Concerns:**
- Workers handle transaction dissemination (network-intensive)
- Primaries handle consensus logic (computation-intensive)
- Horizontal scalability: More workers = more throughput

### 1.2 Core Data Structures

#### Batch
A collection of transactions from a worker:
```rust
struct Batch {
    /// Transactions in this batch
    transactions: Vec<Transaction>,
    /// Worker ID that created this batch
    worker_id: WorkerId,
    /// Timestamp of batch creation
    timestamp: u64,
}
```

**Properties:**
- Fixed maximum size (e.g., 500KB) for network efficiency
- Stored by workers, referenced by digest in headers
- Available on-demand via worker availability protocol

#### Header
Metadata about a batch, proposed by a primary:
```rust
struct Header {
    /// Author (validator public key)
    author: PublicKey,
    /// Round number (monotonically increasing)
    round: u64,
    /// Digest of the batch this header references
    batch_digest: Digest,
    /// References to certificates from previous round (parents)
    parents: Vec<CertificateDigest>,
    /// Timestamp of header creation
    timestamp: u64,
}
```

**Parent Rules:**
- Round R header must reference certificates from round R-1
- Must reference at least 2f+1 certificates from previous round (quorum)
- Round 0 (genesis) has no parents
- Forms causal dependencies in the DAG

#### Certificate
A header certified by quorum of validators:
```rust
struct Certificate {
    /// The header being certified
    header: Header,
    /// Aggregated signatures from validators (at least 2f+1)
    aggregated_signature: AggregatedSignature,
    /// Bitmap indicating which validators signed
    signers: BitVec,
}
```

**Certificate Formation:**
1. Primary broadcasts header to all validators
2. Each validator verifies:
   - Signature is valid
   - Parents exist and are valid
   - Round number is correct (previous_round + 1)
   - No equivocation (no other header from this author in this round)
3. Validator sends vote (signature on header) to author
4. Author collects 2f+1 votes, forms certificate
5. Certificate is broadcast to all validators

**Quorum Threshold:**
```
threshold = ⌊(2n)/3⌋ + 1 = 2f + 1
where n = 3f + 1 (total validators)
```

### 1.3 DAG Construction

**DAG Structure:**
- Vertices: Certificates
- Edges: Parent references (child → parent)
- Organized by rounds (round 0, 1, 2, ...)
- Each validator produces at most one certificate per round

**DAG Properties:**
1. **Acyclic**: Parent references only point to previous rounds
2. **Causal**: If cert A references cert B, then B happened before A
3. **Certified**: Each vertex has 2f+1 signatures guaranteeing data availability
4. **Fault-tolerant**: At least 2f+1 certificates per round (from honest validators)

**DAG Growth:**
```
Round 0:  [C0_v1] [C0_v2] [C0_v3] [C0_v4]  (genesis)
             ↓       ↓       ↓       ↓
Round 1:  [C1_v1] [C1_v2] [C1_v3] [C1_v4]
             ↓       ↓       ↓       ↓
Round 2:  [C2_v1] [C2_v2] [C2_v3] [C2_v4]
             ↓       ↓       ↓       ↓
Round 3:  [C3_v1] [C3_v2] [C3_v3] [C3_v4]
```

Each certificate in round R references at least 2f+1 certificates from round R-1.

### 1.4 Equivocation Detection

**Equivocation** = Byzantine validator producing conflicting certificates in the same round.

**Detection:**
- Each validator maintains a map: `(author, round) → certificate_digest`
- When receiving a certificate, check if author already has a certificate for that round
- If digests differ → equivocation detected

**Handling:**
- Honest validators reject both conflicting certificates
- Do not vote for either certificate
- May trigger slashing/reputation penalties
- Prevents equivocating validator from participating in that round

### 1.5 Batch Availability Protocol

**Problem**: Headers contain only batch digests, not full batch data.

**Solution**: Workers serve batch data on-demand.

**Protocol:**
1. Validator receives certificate with batch digest
2. If batch not locally available, request from author's workers
3. Workers respond with full batch data
4. Validator verifies: hash(batch) == batch_digest
5. Store batch locally for future ordering

**Optimization**: Pre-fetch batches when headers are received (before certification).

### 1.6 Data Guarantees

**Key Guarantee**: If a certificate exists, the batch data is available from at least 2f+1 validators.

**Why**: 
- Certificate requires 2f+1 votes
- Honest validators only vote after verifying batch availability
- Therefore, at least f+1 honest validators have the batch
- At least one honest validator will survive and provide data

This is the fundamental advantage of certified DAG over uncertified DAG (like Mysticeti).

## Part 2: Shoal Consensus Protocol

Shoal is a consensus protocol that operates on the Narwhal certified DAG to determine a total order of certificates (and thus transactions). It achieves low latency through single-round pipelining and adaptive leader selection.

### 2.1 Core Concepts

**Anchor**: A certificate selected as a consensus decision point in a round.

**Leader**: The validator whose certificate is chosen as the anchor for a round.

**Commit**: Making a final decision to include a certificate and its causal history in the ordered output.

**Pipelining**: Processing multiple rounds concurrently, with one anchor per round (not wave-based).

### 2.2 Leader Reputation System

Unlike Bullshark's fixed leader rotation, Shoal uses **reputation-based leader selection**.

#### Reputation State
```rust
struct ReputationState {
    /// Scores for each validator (0.0 to 1.0)
    scores: HashMap<PublicKey, f64>,
    /// Recent performance observations (sliding window)
    recent_performance: VecDeque<PerformanceRecord>,
    /// Configuration parameters
    config: ReputationConfig,
}

struct PerformanceRecord {
    validator: PublicKey,
    round: u64,
    /// Time from round start to certificate appearance
    latency_ms: u64,
    /// Whether certificate was formed successfully
    success: bool,
    timestamp: u64,
}

struct ReputationConfig {
    /// Window size for performance tracking
    window_size: usize,
    /// Decay factor for old observations (0.0 to 1.0)
    decay_factor: f64,
    /// Minimum score (prevents complete exclusion)
    min_score: f64,
}
```

#### Score Calculation

**Initial score**: All validators start at 1.0 (perfect reputation).

**Update formula** (after each round):
```
new_score = decay_factor * old_score + (1 - decay_factor) * round_performance

round_performance = {
    1.0  if certificate appeared quickly and formed
    0.5  if certificate appeared slowly but formed
    0.0  if no certificate appeared (timeout)
}

Latency thresholds:
  - Quick: < target_latency (e.g., 500ms)
  - Slow: >= target_latency
```

**Decay**: Older observations have less weight (exponential decay).

**Minimum bound**: Scores never drop below min_score (e.g., 0.1) to allow recovery.

#### Leader Selection

**Algorithm** (for round R):
```
1. Get all validators' current reputation scores
2. Sort validators by score (descending)
3. Select top validator as leader for round R
4. Tie-breaking: Use deterministic function of (round, validator_id)
```

**Properties:**
- Deterministic: All honest validators compute same leader
- Adaptive: Fast validators selected more often
- Fair: Slow validators still get chances (due to min_score)
- Byzantine-resistant: Byzantine behavior lowers reputation

### 2.3 Consensus Rounds and Anchors

**Round Structure:**
- Each round R has certificates from all (or most) validators
- Shoal selects one certificate per round as the **anchor**
- Anchors are the decision points for ordering

**Anchor Selection** (for round R):
```
1. leader = select_leader_by_reputation(R)
2. anchor = certificate from leader in round R
3. If leader's certificate exists and valid → use it as anchor
4. If leader's certificate missing/invalid → fallback selection
```

**Fallback Selection:**
- If leader's certificate unavailable (Byzantine or network delay)
- Select next-best validator by reputation who has a certificate
- Ensures liveness: Always make progress even if leader fails

### 2.4 Commit Rules

Shoal uses **direct commit** rules for low latency.

#### Direct Commit Rule

A certificate C in round R is **directly committed** if:
1. C is the anchor for round R (selected by reputation)
2. C has a strong path to at least 2f+1 anchors from round R-1

**Strong path**: Certificate A has a path to certificate B if:
- A directly references B (B is in A's parents), OR
- A references certificate C which has a path to B (transitive)

**Why 2f+1 anchors**: Guarantees at least f+1 honest validators agree, ensuring safety.

#### Commit Flow

```
Round R-1: Select anchor A_{R-1} (from 2f+1 certificates)
Round R:   Select anchor A_R
           Check: Does A_R have path to 2f+1 anchors from R-1?
           If yes → Commit A_R and its causal history
           If no → Skip A_R, wait for next round
```

**Pipelining effect**: While committing round R, round R+1 is already being formed.

### 2.5 Certificate Ordering

Once certificates are committed, they must be ordered into a linear sequence.

**Ordering Algorithm:**

1. **Identify Committed Set**: All certificates that have been committed up to current round

2. **Topological Sort**: Order certificates respecting DAG edges (causal dependencies)
   - If cert A references cert B, then B must come before A in the order

3. **Deterministic Tie-Breaking**: When multiple certificates have no causal relationship:
   - Sort by: (round, author_id)
   - Lower round first
   - Within same round, lexicographic order of author public keys

4. **Extract Transactions**: Flatten batches from ordered certificates into transaction sequence

**Ordering Properties:**
- **Deterministic**: All honest validators compute same order
- **Causal**: Preserves happens-before relationships from DAG
- **Fair**: All certificates eventually included (liveness)

### 2.6 Prevalent Responsiveness

**Problem in Bullshark**: Fixed timeouts for waiting for certificates cause latency even when network is fast.

**Shoal's Solution**: **Prevalent responsiveness** - respond to actual network conditions without timeouts.

**Mechanism:**
1. **Reputation-based expectations**: Know which validators are typically fast
2. **Dynamic waiting**: Don't wait for slow validators' certificates
3. **Immediate progress**: Commit as soon as fast leaders' certificates appear
4. **Fallback only when needed**: Use timeouts only for true unavailability

**Result**: Latency matches actual network performance, not artificial timeout values.

### 2.7 Safety and Liveness

**Safety**: No two honest validators commit different orders.

**Proof sketch**:
- Certificate requires 2f+1 votes
- Commit requires path to 2f+1 anchors
- At most f Byzantine validators
- Therefore, at least f+1 honest validators agree on commits
- Honest validators use deterministic ordering
- Therefore, all honest validators compute same order

**Liveness**: All certificates eventually committed.

**Proof sketch**:
- At least 2f+1 validators are honest
- Honest validators follow protocol and produce certificates
- Therefore, at least 2f+1 certificates per round
- Reputation ensures some honest validator selected as leader
- Direct commit rule eventually satisfied
- Therefore, progress guaranteed

**Partial synchrony assumption**: After Global Stabilization Time (GST), network has bounded delay. This ensures liveness.

## Part 3: Protocol Integration

### 3.1 Combined Architecture

```
┌─────────────────────────────────────────────┐
│           Modal Sequencer                   │
│  (Transaction submission & state machine)   │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Shoal Consensus Layer               │
│  - Anchor selection (reputation-based)      │
│  - Commit rules (direct commit)             │
│  - Certificate ordering (topological sort)  │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Narwhal DAG Layer                   │
│  - Certificate formation (2f+1 quorum)      │
│  - DAG construction and validation          │
│  - Batch availability protocol              │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Network Communication               │
│  - Certificate gossip                       │
│  - Vote collection                          │
│  - Batch requests                           │
└─────────────────────────────────────────────┘
```

### 3.2 Validator Lifecycle

**Initialization**:
1. Load validator keys (public/private keypair)
2. Connect to network, discover peers
3. Sync DAG from other validators (catch up)
4. Initialize reputation state (all validators at 1.0)
5. Start worker threads for batch collection
6. Start primary thread for consensus

**Normal Operation** (per round):
1. **Workers**: Collect transactions → form batches
2. **Primary**: Create header referencing batch + parents from prev round
3. **Primary**: Broadcast header to all validators
4. **All validators**: Receive header → verify → vote (sign)
5. **Primary**: Collect 2f+1 votes → form certificate
6. **Primary**: Broadcast certificate to all validators
7. **Consensus**: Select anchor for this round (based on reputation)
8. **Consensus**: Check commit rule → commit if satisfied
9. **Consensus**: Update reputation based on round performance
10. **Consensus**: Output ordered transactions to state machine

**Recovery** (after crash):
1. Reload DAG from persistent storage
2. Reload reputation state
3. Sync missing certificates from peers
4. Resume from latest committed round

### 3.3 Configuration Parameters

```rust
struct ShoalConfig {
    // Narwhal parameters
    batch_size: usize,              // Max transactions per batch (e.g., 1000)
    max_batch_bytes: usize,         // Max batch size in bytes (e.g., 500KB)
    workers_per_validator: usize,   // Number of worker threads (e.g., 4)
    
    // Consensus parameters
    target_latency_ms: u64,         // Target for "fast" certificate (e.g., 500ms)
    commit_history_depth: usize,    // How many rounds to keep in memory (e.g., 1000)
    
    // Reputation parameters
    reputation_window: usize,       // Performance observation window (e.g., 100)
    reputation_decay: f64,          // Decay factor for old observations (e.g., 0.9)
    reputation_min_score: f64,      // Minimum reputation score (e.g., 0.1)
    
    // Network parameters
    timeout_ms: u64,                // Fallback timeout (e.g., 5000ms)
    gossip_fanout: usize,           // Number of peers to gossip to (e.g., 8)
}
```

## Part 4: Byzantine Fault Tolerance

### 4.1 Attack Vectors and Mitigations

**Attack 1: Equivocation (double-signing)**
- Attacker creates two conflicting headers in same round
- Mitigation: Equivocation detection, reject both certificates

**Attack 2: Withholding votes**
- Byzantine validator refuses to vote for others' headers
- Mitigation: Only need 2f+1 votes, f Byzantine validators can't stop quorum

**Attack 3: Withholding certificates**
- Byzantine validator forms certificate but doesn't broadcast
- Mitigation: Gossip protocol ensures propagation, fallback leader selection

**Attack 4: Invalid parent references**
- Byzantine validator references non-existent or wrong-round parents
- Mitigation: Validators verify parent references before voting

**Attack 5: Batch unavailability**
- Byzantine validator creates header but doesn't store batch
- Mitigation: Certified DAG guarantees 2f+1 validators have batch data

**Attack 6: Reputation manipulation**
- Byzantine validator tries to inflate its reputation
- Mitigation: Reputation based on objective metrics (latency), observed by all

### 4.2 Safety Guarantees

**Theorem**: With at most f Byzantine validators out of n = 3f+1, Shoal guarantees:
1. **Agreement**: All honest validators commit the same order
2. **Validity**: Committed transactions were actually proposed
3. **Integrity**: Each transaction appears at most once in the order

**Assumptions**:
- At most f Byzantine validators
- Certified DAG with 2f+1 quorum
- Partial synchrony (bounded delay after GST)
- Strong cryptography (signatures unforgeable)

## Part 5: Performance Characteristics

### 5.1 Expected Performance

**Throughput**: 125,000+ transactions per second
- Assumes: 1KB average transaction size, good network conditions
- Scales with number of workers and network bandwidth

**Latency**: ~1.2 seconds average
- Round-trip time for certificate formation (~400ms)
- Consensus commit decision (~400ms)
- Ordering and execution (~400ms)

**Resource Usage**:
- CPU: Moderate (signature verification, DAG maintenance)
- Memory: Grows with DAG size (configurable history depth)
- Network: High bandwidth for transaction dissemination

### 5.2 Scalability

**Horizontal Scaling**:
- More workers per validator → higher throughput
- Limited by network bandwidth and CPU for signatures

**Validator Scaling**:
- Performance degrades gracefully with more validators
- 2f+1 quorum means more validators = more signatures
- Shoal tested with 50+ validators in production (Aptos)

## Part 6: Academic References

### Primary Papers

1. **Narwhal and Tusk: A DAG-based Mempool and Efficient BFT Consensus**
   - Authors: Danezis et al.
   - arXiv: 2105.11827
   - Core reference for Narwhal DAG construction

2. **Bullshark: DAG BFT Protocols Made Practical**
   - Authors: Spiegelman et al.
   - Context for understanding wave-based consensus (which Shoal improves)

3. **Shoal: Reducing Bullshark Latency on the Aptos Blockchain**
   - Authors: Aptos Labs
   - Medium article: Details on pipelining and reputation

### Key Concepts

- **Byzantine Fault Tolerance**: Protocols that tolerate arbitrary failures
- **Partial Synchrony**: Network model with eventual bounded message delay
- **DAG-based Consensus**: Using directed acyclic graphs for causal ordering
- **Quorum Certificates**: Proofs of agreement from 2f+1 validators

### Production Deployments

- **Aptos Blockchain**: Uses Shoal consensus in production
- **Performance**: Demonstrated 160K TPS with sub-2s latency in testnet

---

**Document Status**: Technical Specification v1.0  
**Last Updated**: 2025-10-30  
**Target Implementation**: modal-sequencer  

