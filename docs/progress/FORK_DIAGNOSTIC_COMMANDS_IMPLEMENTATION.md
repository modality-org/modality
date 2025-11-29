# Fork Diagnostic CLI Commands Implementation

## Overview

Added two new CLI commands to help diagnose blockchain forks between nodes:

1. `modal node inspect block <index>` - Inspect a specific block by index
2. `modal node compare <peer>` - Compare local chain with a remote peer's chain

## Implementation Details

### 1. Block Inspection Command

**Command:** `modal node inspect block <index>`

**Purpose:** Display detailed information about a specific block at a given index, including detection of multiple blocks at the same height (fork detection).

**Features:**
- Shows block status (Canonical, Orphaned, or Pending)
- Displays full block details: hash, previous hash, timestamp, epoch, difficulty, nonce, nominated peer, miner number
- For orphaned blocks, shows orphan reason and competing hash
- Detects forks when multiple blocks exist at the same index
- Works offline (read-only datastore access)

**Example Usage:**
```bash
cd testnet1
modal node inspect block 45379
```

**Example Output:**
```
📦 Block 45379 Details
==================

Status: ✓ Canonical
Hash: abc123...
Previous Hash: def456...
Timestamp: 1701234567 (2023-11-29 10:15:67 UTC)
Epoch: 1
Difficulty: 1000000
Nonce: 42
Nominated Peer: 12D3KooW...
Miner Number: 0

---

Status: ⚠️  Orphaned
Hash: xyz789...
Previous Hash: def456...
Timestamp: 1701234568 (2023-11-29 10:15:68 UTC)
Epoch: 1
Difficulty: 1000000
Nonce: 43
Nominated Peer: 12D3KooW...
Miner Number: 1
Orphan Reason: fork_choice_rule_violation
Competing Hash: abc123...

⚠️  WARNING: 2 blocks found at index 45379 (fork detected)
```

### 2. Chain Comparison Command

**Command:** `modal node compare <peer>`

**Purpose:** Compare the local blockchain with a remote peer's chain to identify fork points and determine which chain is heavier.

**Features:**
- Accepts either a full multiaddr or a peer ID (if found in bootstrappers)
- Connects to the remote peer via reqres protocol
- Queries remote chain info (length, cumulative difficulty, tip hash)
- Finds common ancestor using exponential backoff checkpoint algorithm
- Identifies fork divergence points
- Recommends which chain to follow based on cumulative difficulty
- Shows actionable next steps

**Example Usage:**
```bash
# Using multiaddr
cd testnet1
modal node compare /ip4/1.2.3.4/tcp/4040/ws/p2p/12D3KooWEA6dRWvK1vutRDxKfdPZZr7ycHvQNWrDGZZQbiE6YibZ

# Using peer ID (if in bootstrappers)
modal node compare 12D3KooWEA6dRWvK1vutRDxKfdPZZr7ycHvQNWrDGZZQbiE6YibZ

# With custom timeout
modal node compare --timeout-secs 60 /ip4/1.2.3.4/tcp/4040/ws/p2p/12D3...
```

**Example Output:**
```
🔍 Comparing chains with peer 12D3KooWEA6dRWvK1vutRDxKfdPZZr7ycHvQNWrDGZZQbiE6YibZ

🔗 Connecting to peer...
   Connected!

📡 Requesting chain info...
🔎 Finding common ancestor...

📊 Chain Comparison
==================

Local Chain:
  Length: 45380 blocks
  Orphans: 23 blocks
  Cumulative Difficulty: 98234567890
  Tip Hash: abc123...
  Tip Index: 45379

Remote Chain:
  Length: 45385 blocks
  Cumulative Difficulty: 98734567890
  Tip Hash: xyz789...

✓ Common Ancestor: Block 45377
  Hash: def456...

⚠️  FORK DETECTED
  Local diverged: 2 blocks (from 45378 to 45379)
  Remote diverged: 7 blocks (from 45378 to 45384)

⚠️  Remote chain is heavier (ahead by 500000000 difficulty)
   Consider syncing to adopt the heavier chain:
   modal node sync
```

## Files Modified

1. **rust/modal/src/cmds/node/inspect.rs**
   - Added `block_index: Option<u64>` to `Opts` struct
   - Added `"block"` case to command match
   - Implemented `inspect_block_by_index()` function

2. **rust/modal/src/cmds/node/compare.rs** (new file)
   - Full implementation of chain comparison command
   - Uses Node::from_config() for network communication
   - Implements exponential backoff checkpoint algorithm for finding common ancestor
   - Parses peer addresses (multiaddr or peer ID)

3. **rust/modal/src/cmds/node/mod.rs**
   - Added `pub mod compare;`

4. **rust/modal/src/main.rs**
   - Added `Compare(cmds::node::compare::Opts)` to `NodeCommands` enum
   - Added `NodeCommands::Compare(opts) => cmds::node::compare::run(opts).await?` to match statement

## Technical Implementation Notes

### Block Inspection
- Uses `MinerBlock::find_by_index()` to retrieve all blocks at a given index
- Handles multiple blocks gracefully (fork detection)
- Formats timestamps using chrono for human-readable output
- Read-only access, no network required

### Chain Comparison
- Creates temporary Node instance for network communication
- Uses existing `/data/miner_block/chain_info` reqres endpoint for remote chain info
- Uses existing `/data/miner_block/find_ancestor` reqres endpoint with checkpoint algorithm
- Exponential backoff: checks [tip, tip-1, tip-2, tip-4, tip-8, ...] for efficient ancestor finding
- Calculates cumulative difficulty for both chains
- Provides actionable recommendations based on fork choice rules

### Error Handling
- Graceful timeout handling (default 30s, configurable)
- Clear error messages for connection failures
- Validates peer addresses and provides helpful error messages
- Automatically disconnects from peer after operation

## Testing

The commands have been built successfully and are available in the CLI:

```bash
modal node inspect block <index>
modal node compare <peer>
```

## Use Cases

### Diagnosing Fork Between Two Nodes

```bash
# On testnet1, compare with testnet2
ssh testnet1
cd ~/testnet1
~/.modality/bin/modal node compare /ip4/<testnet2-ip>/tcp/4040/ws/p2p/<testnet2-peer-id>
```

### Inspecting Specific Block for Hash Mismatch

```bash
# Check block 45379 on both nodes
ssh testnet1 "cd ~/testnet1 && ~/.modality/bin/modal node inspect block 45379"
ssh testnet2 "cd ~/testnet2 && ~/.modality/bin/modal node inspect block 45379"
```

### Finding Last Common Block

The `compare` command automatically finds the last common block using binary search with exponential backoff, making it efficient even for large forks.

## Benefits

1. **Quick Fork Diagnosis**: Instantly identify where two nodes diverged
2. **Offline Block Inspection**: Check block details without running the node
3. **Actionable Recommendations**: Clear guidance on which chain to follow
4. **Efficient Algorithm**: Exponential backoff finds common ancestor in O(log n) requests
5. **Flexible Input**: Accept both multiaddr and peer ID for convenience
6. **Production Ready**: Proper error handling, timeouts, and resource cleanup

## Future Enhancements

Potential improvements for future iterations:

1. **Multi-Peer Comparison**: Compare with multiple peers simultaneously
2. **Automatic Sync**: Option to automatically sync to heavier chain
3. **Fork Visualization**: ASCII graph showing fork structure
4. **Historical Analysis**: Show fork statistics over time
5. **Alert Integration**: Notify when forks are detected
6. **Block Range Inspection**: Inspect multiple blocks at once

## Conclusion

These diagnostic tools significantly improve the ability to diagnose and resolve blockchain forks in the Modality network. They provide clear visibility into chain state and actionable recommendations for resolving divergence issues.

