# Message Queue Communication Tests

**Added**: October 30, 2025  
**Location**: `rust/modal-sequencer-consensus/tests/integration_tests.rs`

## Overview

Two new integration tests demonstrate multi-validator Shoal consensus using in-process message queue communication via `tokio::mpsc` channels.

## Tests

### 1. `test_message_queue_communication`

**Purpose**: Demonstrates basic certificate broadcasting between validators.

**Setup**:
- 4 validators with independent DAG and consensus state
- Shared message queue (`tokio::mpsc::unbounded_channel`)
- Message processor task that broadcasts certificates to all validators

**Flow**:
1. Each validator proposes a genesis certificate
2. Certificate is processed locally
3. Certificate is broadcast via message queue
4. All other validators receive and process it
5. Verify all validators have all 4 genesis certificates

**Key Assertions**:
- All validators receive exactly 4 genesis certificates
- Each validator commits at least 1 certificate

### 2. `test_message_queue_round_progression`

**Purpose**: Demonstrates multi-round consensus with parent references.

**Setup**: Same as above

**Flow**:
1. **Round 0** (Genesis):
   - All 4 validators propose genesis certificates
   - Certificates broadcast via message queue
   - All validators advance to round 1

2. **Round 1** (With Parents):
   - All validators propose new certificates
   - Each references all 4 round 0 certificates as parents
   - Certificates broadcast and processed
   - Verify DAG structure and parent relationships

**Key Assertions**:
- All validators have 4 round 0 certificates
- All validators have 4 round 1 certificates
- Round 1 certificates correctly reference all round 0 parents

## Architecture

```rust
// Message type for certificate broadcasting
type Message = (usize, Certificate);  // (from_validator_id, certificate)

// Create channel
let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

// Message processor (spawned task)
tokio::spawn(async move {
    while let Some((from, cert)) = rx.recv().await {
        // Broadcast to all validators except sender
        for (i, (_dag, consensus)) in validators.iter().enumerate() {
            if i != from {
                consensus.process_certificate(cert.clone()).await;
            }
        }
    }
});

// Each validator broadcasts
tx.send((validator_id, certificate)).unwrap();
```

## Benefits

1. **Realistic Testing**: Simulates distributed consensus without network complexity
2. **Fast Execution**: No network I/O, runs in ~200ms
3. **Deterministic**: No network jitter or timing issues
4. **Debuggable**: Easy to add logging and inspect state
5. **Foundation**: Can evolve to real networking layer

## Comparison with Other Tests

| Test Type | Communication | Validators | Realism |
|-----------|--------------|------------|---------|
| Unit tests | Direct function calls | Simulated | Low |
| Existing integration | Manual certificate passing | Simulated | Medium |
| **New message queue** | **Async message passing** | **Independent instances** | **High** |
| Network tests (future) | TCP/UDP sockets | Separate processes | Highest |

## Output Example

```
=== Round 0: Genesis ===
Validator 0 received 4 genesis certificates via message queue
Validator 0 committed 1 certificates
Validator 1 received 4 genesis certificates via message queue
Validator 1 committed 1 certificates
[...]

=== Round 1: With Parents ===
Validator 0: Round 0: 4 certs, Round 1: 4 certs
Validator 0 committed 1 total certificates
[...]
```

## Future Enhancements

These tests can be extended to add:

1. **Transaction Broadcasting**:
   ```rust
   enum Message {
       Batch(Batch),
       Header(Header),
       Vote(Vote),
       Certificate(Certificate),
   }
   ```

2. **Vote Collection**:
   - Validators broadcast headers
   - Others send votes back
   - Proposer collects votes to form certificate

3. **Network Delays**:
   ```rust
   tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 100)).await;
   ```

4. **Byzantine Behavior**:
   - Drop messages randomly
   - Send duplicate certificates
   - Send invalid certificates

5. **Performance Testing**:
   - Measure throughput with message queues
   - Compare against direct calls
   - Profile async overhead

## Related Files

- `src/communication/same_process.rs` - Existing same-process communication (for legacy consensus)
- `tests/integration_tests.rs` - All integration tests including these new ones
- `src/shoal/consensus.rs` - Shoal consensus engine being tested

## Test Results

```bash
$ cargo test --test integration_tests test_message_queue

running 2 tests
test test_message_queue_communication ... ok
test test_message_queue_round_progression ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured
```

---

**Status**: ✅ Complete and Passing  
**Total Integration Tests**: 12 (up from 10)

