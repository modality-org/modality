<!-- 1b69aaa7-079e-4c77-9b6a-ddf95930747a af421da5-23c3-4834-9dac-8578db431eaa -->
# Shoal Consensus Implementation Plan

## Current State

✅ **IMPLEMENTATION COMPLETE**

All phases have been implemented and tested.

## Implementation Phases

### Phase 1: ValidatorBlock Creation and Persistence ✅

Updated consensus loop in `modal-node/src/actions/validator/consensus.rs` to:

1. Create a `ValidatorBlock` each round with proper fields (peer_id, round_id, prev_round_certs)
2. Generate opening/closing signatures using node keypair
3. Save draft block to `validator_active` store via `save_to_active()`
4. Update `ConsensusMetadata` with round progress

Key files:

- `rust/modal-node/src/actions/validator/consensus.rs` - main consensus loop
- `rust/modal-datastore/src/models/validator/block.rs` - ValidatorBlock creation

### Phase 2: Gossip Broadcasting ✅

Integrated `NodeCommunication` for gossip broadcasting:

1. Used existing `NodeCommunication` struct implementing the `Communication` trait
2. `broadcast_draft_block()` - publishes to `/consensus/block/draft` topic
3. `broadcast_certified_block()` - publishes to `/consensus/block/cert` topic
4. `send_block_ack()` - sends ack via request-response
5. Wired gossip communication into consensus loop

Key files:

- `rust/modal-node/src/consensus/node_communication.rs` - Communication implementation
- `rust/modal-node/src/actions/validator/consensus.rs`
- `rust/modal-node/src/actions/validator/mod.rs` - subscribed to validator gossip topics

### Phase 3: Ack Collection and Certificate Formation ✅

Implemented ack/signature collection:

1. Created `AckCollector` struct to track received draft blocks and acks
2. Handle incoming draft blocks from gossip, validate, and send acks
3. Track received acks per round
4. When 2f+1 acks received, form certificate and attach to block
5. Uses `ValidatorBlock.acks` field for ack storage and `ValidatorBlock.cert` for certificate

Key files:

- `rust/modal-node/src/actions/validator/ack_collector.rs` (new)
- `rust/modal-datastore/src/models/validator/block.rs` - Ack struct

### Phase 4: Certified Block Processing ✅

Handle certified blocks:

1. Receive certified blocks via gossip handler
2. Validate certificate signatures against known validator set via `validate_certificate()`
3. Save certified blocks via `save_certified_block()`
4. Promote certified blocks: `save_to_active()` then `promote_to_final()`
5. Run periodic finalization task via `run_finalization_task()`

Key files:

- `rust/modal-node/src/gossip/consensus/block/cert.rs`
- `rust/modal-node/src/actions/validator/ack_collector.rs` - certificate validation

### Phase 5: Round Advancement and Finalization ✅

Complete the consensus cycle:

1. Round advances automatically every 2 seconds
2. Update `current_round` in datastore for status page
3. Run finalization task every 5 rounds to move certified blocks to `validator_final`
4. Cleanup old data from ack collector
5. Status page reads from stores and displays finalized rounds

Key files:

- `rust/modal-node/src/actions/validator/consensus.rs`
- `rust/modal-node/src/status_server.rs` - already reads from stores

## Testing Strategy

1. ✅ Unit tests for each component (ack collector, communication)
2. Integration test with 3+ in-memory validators (future work)
3. Multi-node test using devnet configuration (future work)

## Key Design Decisions

- ValidatorBlocks are the primary consensus unit (not Narwhal Certificates directly)
- Gossip for broadcast, request-response for targeted acks
- 2f+1 threshold for certificate formation (f = floor((n-1)/3))
- Independent of mining chain (no miner block references)

## Completed To-dos

- [x] Create ValidatorBlocks in consensus loop with signatures and persistence
- [x] Implement GossipCommunication trait for broadcasting blocks
- [x] Build AckCollector for gathering signatures and forming certs
- [x] Handle certified blocks: validation, DAG insertion, finalization
- [x] Complete round advancement and status page integration

## Files Modified/Created

### New Files
- `rust/modal-node/src/actions/validator/ack_collector.rs` - Ack collection and certificate formation

### Modified Files
- `rust/modal-node/src/actions/validator/consensus.rs` - Full consensus loop implementation
- `rust/modal-node/src/actions/validator/hybrid.rs` - Pass keypair, swarm, consensus_tx
- `rust/modal-node/src/actions/validator/mod.rs` - Wire up components, subscribe to gossip topics
- `rust/modal-node/src/node/mod.rs` - Add `get_consensus_tx()` getter

