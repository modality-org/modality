# DAG Synchronization - Request/Response Protocol

This document describes the DAG synchronization protocol used to sync certificates between nodes in the Shoal consensus.

## Overview

The sync protocol enables nodes to:
- Request missing certificates from peers
- Sync their DAG to match other nodes
- Handle certificate dependencies (parent references)
- Catch up after being offline or slow

## Protocol Components

### 1. Sync Messages

Located in `modal-sequencer-consensus/src/narwhal/sync.rs`

#### SyncRequest

Enum representing requests a node can make:

```rust
pub enum SyncRequest {
    /// Request certificates by digest
    GetCertificates { digests: Vec<CertificateDigest> },
    
    /// Request all certificates in a specific round
    GetCertificatesInRound { round: u64 },
    
    /// Request certificates in a range of rounds
    GetCertificatesInRange { start_round: u64, end_round: u64 },
    
    /// Request a batch by digest
    GetBatch { digest: BatchDigest },
    
    /// Request multiple batches by digest
    GetBatches { digests: Vec<BatchDigest> },
    
    /// Request the highest round number
    GetHighestRound,
    
    /// Request missing certificates based on parent references
    GetMissingCertificates {
        known_digests: Vec<CertificateDigest>,
        up_to_round: u64,
    },
}
```

#### SyncResponse

Enum representing responses:

```rust
pub enum SyncResponse {
    /// Certificates response
    Certificates {
        certificates: Vec<Certificate>,
        has_more: bool,
    },
    
    /// Batch response
    Batches {
        batches: Vec<Batch>,
    },
    
    /// Highest round response
    HighestRound {
        round: u64,
    },
    
    /// Error response
    Error {
        message: String,
    },
    
    /// Empty response (no data found)
    Empty,
}
```

### 2. Request Handler (DAG)

Located in `modal-sequencer-consensus/src/narwhal/dag.rs`

The DAG implements `handle_sync_request()` which processes incoming requests:

```rust
pub fn handle_sync_request(&self, request: SyncRequest) -> SyncResponse {
    // Handles all types of sync requests and returns appropriate responses
}
```

**Key features:**
- Limits response sizes (max 1000 certificates per response)
- Sets `has_more` flag when more data is available
- Returns `Empty` when no data found
- Returns `Error` for batch requests (handled by Workers)

**Helper methods:**
```rust
// Check if certificate has missing parents
pub fn get_missing_parents(&self, cert: &Certificate) -> Vec<CertificateDigest>

// Check if all parents are available
pub fn has_all_parents(&self, cert: &Certificate) -> bool

// Get list of missing certificates up to a round
pub fn get_missing_certificates_up_to_round(&self, up_to_round: u64) -> Vec<CertificateDigest>
```

### 3. Sync Client

Located in `modal-sequencer-consensus/src/narwhal/sync_client.rs`

Provides high-level methods for requesting data from peers:

```rust
pub struct SyncClient {
    dag: Arc<RwLock<DAG>>,
}
```

**Methods:**

#### sync_with_peer()
Syncs the entire DAG with a peer:
```rust
pub async fn sync_with_peer<F, Fut>(&self, request_fn: F) -> Result<SyncStats>
where
    F: Fn(SyncRequest) -> Fut,
    Fut: std::future::Future<Output = Result<SyncResponse>>,
```

**Algorithm:**
1. Get peer's highest round
2. Request certificates in batches of 10 rounds
3. Insert certificates into local DAG
4. Skip duplicates gracefully
5. Return statistics

#### request_certificates()
Requests specific certificates by digest:
```rust
pub async fn request_certificates<F, Fut>(
    &self,
    digests: Vec<CertificateDigest>,
    request_fn: F,
) -> Result<Vec<Certificate>>
```

#### sync_missing_parents()
Syncs missing parents for a certificate:
```rust
pub async fn sync_missing_parents<F, Fut>(
    &self,
    cert: &Certificate,
    request_fn: F,
) -> Result<bool>
```

Returns `true` if all parents were successfully synced.

#### sync_gaps()
Detects and syncs gaps in the DAG:
```rust
pub async fn sync_gaps<F, Fut>(
    &self,
    up_to_round: u64,
    request_fn: F,
) -> Result<SyncStats>
```

### 4. ShoalSequencer Integration

Located in `modal-sequencer/src/shoal_sequencer.rs`

The ShoalSequencer exposes sync methods for application use:

```rust
// Handle incoming sync requests from peers
pub async fn handle_sync_request(&self, request: SyncRequest) -> SyncResponse

// Sync with a peer
pub async fn sync_with_peer<F, Fut>(&self, request_fn: F) -> Result<SyncStats>

// Request specific certificates
pub async fn request_certificates<F, Fut>(
    &self,
    digests: Vec<CertificateDigest>,
    request_fn: F,
) -> Result<Vec<Certificate>>

// Sync and process a certificate (with parent resolution)
pub async fn sync_and_process_certificate<F, Fut>(
    &self,
    cert: Certificate,
    request_fn: F,
) -> Result<Vec<Transaction>>

// Utility methods
pub async fn get_highest_round(&self) -> u64
pub async fn has_complete_round(&self, round: u64) -> bool
```

## Usage Examples

### Example 1: Handle Sync Request

```rust
// Node receives a sync request from a peer
let request = SyncRequest::certificates_in_round(42);
let response = sequencer.handle_sync_request(request).await;

match response {
    SyncResponse::Certificates { certificates, .. } => {
        // Send certificates back to peer
        send_to_peer(certificates).await?;
    }
    SyncResponse::Empty => {
        // No certificates found
    }
    _ => {}
}
```

### Example 2: Sync with a Peer

```rust
// Create request function that talks to peer
let request_fn = |req: SyncRequest| async move {
    // Serialize request
    let req_bytes = serde_json::to_vec(&req)?;
    
    // Send to peer and get response
    let resp_bytes = network.send_to_peer(peer_id, req_bytes).await?;
    
    // Deserialize response
    let response: SyncResponse = serde_json::from_slice(&resp_bytes)?;
    Ok(response)
};

// Sync with peer
let stats = sequencer.sync_with_peer(request_fn).await?;
println!("Synced {} certificates", stats.certificates_synced);
```

### Example 3: Process Certificate with Parent Sync

```rust
// Receive a certificate from the network
let cert = receive_certificate().await?;

// Sync missing parents and process
let transactions = sequencer.sync_and_process_certificate(cert, request_fn).await?;

// Process transactions
for tx in transactions {
    execute_transaction(tx).await?;
}
```

### Example 4: Periodic Sync Loop

```rust
// Background task that syncs with peers periodically
loop {
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    for peer in &committee.validators {
        if peer.public_key == our_key {
            continue; // Skip self
        }
        
        let peer_id = peer.public_key;
        let request_fn = |req| network_request(peer_id, req);
        
        match sequencer.sync_with_peer(request_fn).await {
            Ok(stats) => {
                log::info!("Synced with peer {}: {} certs", peer_id, stats.certificates_synced);
            }
            Err(e) => {
                log::warn!("Failed to sync with peer {}: {}", peer_id, e);
            }
        }
    }
}
```

## Request Function Pattern

The sync methods use a generic request function pattern to allow flexible transport layers:

```rust
F: Fn(SyncRequest) -> Fut,
Fut: std::future::Future<Output = anyhow::Result<SyncResponse>>,
```

This allows you to implement sync over:
- HTTP/REST
- gRPC
- libp2p
- In-process (for testing)
- Any custom protocol

**Example implementations:**

```rust
// HTTP implementation
let request_fn = |req: SyncRequest| async move {
    let response = client
        .post(format!("{}/sync", peer_url))
        .json(&req)
        .send()
        .await?;
    Ok(response.json().await?)
};

// libp2p implementation
let request_fn = |req: SyncRequest| async move {
    let req_bytes = bincode::serialize(&req)?;
    let response = swarm
        .send_request(&peer_id, req_bytes)
        .await?;
    Ok(bincode::deserialize(&response)?)
};

// In-process (testing)
let request_fn = |req: SyncRequest| async move {
    let dag = peer_dag.read().await;
    Ok(dag.handle_sync_request(req))
};
```

## Performance Characteristics

### Response Limits
- Max 1000 certificates per response
- Batched requests for large ranges
- `has_more` flag indicates additional data

### Batching Strategy
- Requests 10 rounds at a time
- Balances: request count vs response size
- Adjustable via `batch_size` variable

### Deduplication
- DAG handles duplicate insertions gracefully
- Certificates already present are skipped
- No data corruption from duplicates

### Statistics
```rust
pub struct SyncStats {
    pub certificates_synced: usize,
    pub certificates_failed: usize,
}
```

## Error Handling

### Client Errors
- Network timeouts: Retry with backoff
- Invalid responses: Log and skip peer
- Missing parents: Recursively sync parents
- DAG insertion failures: Skip certificate

### Server Errors
- Invalid requests: Return `SyncResponse::Error`
- Internal errors: Return `SyncResponse::Error`
- No data found: Return `SyncResponse::Empty`

## Security Considerations

### Validation
- Verify certificate signatures before insertion
- Check parent references exist
- Validate round numbers are sequential
- Ensure authors are in committee

### Rate Limiting
- Limit requests per peer
- Limit response sizes
- Timeout long-running requests

### DoS Protection
- Cap max certificates per response (1000)
- Validate round ranges are reasonable
- Reject malformed requests early

## Testing

Comprehensive test suite in `modal-sequencer-consensus/tests/sync_tests.rs`:

- `test_sync_request_get_certificates` - Request specific certificates
- `test_sync_request_get_round` - Request certificates by round
- `test_sync_request_get_range` - Request certificate ranges
- `test_sync_request_highest_round` - Query highest round
- `test_sync_request_missing_certificates` - Find missing certs
- `test_sync_client_with_peer` - Full sync with peer
- `test_get_missing_parents` - Detect missing parents
- `test_has_all_parents` - Verify parent availability
- `test_sync_missing_parents` - Sync parent certificates
- `test_sync_gaps` - Sync with gaps in DAG

All tests use in-process request functions for fast, deterministic testing.

## Future Enhancements

1. **Bloom Filters**: Efficiently identify missing certificates
2. **Priority Sync**: Prioritize recent rounds over old ones
3. **Parallel Sync**: Sync from multiple peers simultaneously
4. **Delta Sync**: Only sync changes since last sync
5. **Compression**: Compress certificate batches
6. **Merkle Proofs**: Verify certificate completeness
7. **Peer Reputation**: Track reliable sync peers
8. **Adaptive Batch Sizes**: Adjust based on network conditions

## See Also

- [DAG_PERSISTENCE.md](./DAG_PERSISTENCE.md) - DAG persistence layer
- [SHOAL_SPECIFICATION.md](./SHOAL_SPECIFICATION.md) - Shoal consensus protocol
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Overall system architecture

