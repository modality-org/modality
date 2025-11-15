# Network Partition and Recovery Example

This example demonstrates how the Modality network handles network partitions and recovers from validator failures, showcasing Byzantine Fault Tolerance (BFT) resilience properties.

## Overview

Network partitions occur when validators become temporarily unreachable due to:
- Network failures or connectivity issues
- Node crashes or restarts
- Hardware failures
- Malicious actors attempting denial-of-service

This example simulates these scenarios and demonstrates how the consensus protocol maintains **safety** and **liveness** within Byzantine Fault Tolerance guarantees.

## Byzantine Fault Tolerance Background

### Key Concepts

**Byzantine Fault Tolerance (BFT)**: A system's ability to continue operating correctly even when some validators behave maliciously or fail arbitrarily.

**Safety**: The system never commits incorrect state or violates consensus rules. All honest validators agree on the same transaction order.

**Liveness**: The system continues to make progress and commit new transactions when possible.

**Byzantine Threshold**: Maximum number of faulty validators the system can tolerate:
- Formula: `f = (n - 1) / 3` where `n` is total validators
- For n=4: f=1 (can tolerate 1 Byzantine validator)
- For n=7: f=2 (can tolerate 2 Byzantine validators)

**Quorum**: Minimum validators needed for consensus decisions:
- Formula: `quorum = 2f + 1`
- For n=4, f=1: quorum = 3 validators
- This ensures at least `f+1` honest validators participate

### Why These Numbers?

The requirement of `n ≥ 3f + 1` comes from the need to:
1. Have enough honest validators to form quorum: `2f + 1`
2. Ensure quorum always contains `f + 1` honest validators
3. Tolerate up to `f` Byzantine validators

Example with n=4, f=1:
- Total validators: 4
- Byzantine validators: ≤1
- Honest validators: ≥3
- Quorum requirement: 3
- Result: Quorum is guaranteed to have at least 2 honest validators

## Test Scenarios

### Network Configuration

This example uses a 4-validator network:
- **n = 4** validators (node1, node2, node3, node4)
- **f = 1** Byzantine tolerance
- **Quorum = 2f + 1 = 3** validators required

### Scenario 1: Single Node Partition (Within Tolerance)

**What happens:**
- One validator (node4) is killed to simulate network partition
- Three validators remain active
- Network has exactly the quorum requirement (3 validators)

**Expected behavior:**
- ✓ Consensus continues with 3 validators
- ✓ Transactions can still be committed
- ✓ System demonstrates liveness
- ✓ Remaining validators maintain consistent state

**Why it works:**
With 3 active validators and quorum = 3, the network has exactly enough validators to continue consensus. This demonstrates tolerance of `f=1` failures.

### Scenario 2: Node Recovery and Catch-up

**What happens:**
- Previously partitioned node (node4) is restarted
- Node rejoins the network
- Node synchronizes missed state

**Expected behavior:**
- ✓ Node successfully reconnects to peers
- ✓ Node syncs blocks and state from active validators
- ✓ Node resumes participation in consensus
- ✓ Network returns to full strength (4 validators)

**Why it matters:**
This demonstrates the network's self-healing properties. Validators can temporarily fail and recover without manual intervention or state corruption.

### Scenario 3: Two-Node Partition (Exceeds Threshold)

**What happens:**
- Two validators (node3 and node4) are killed
- Only two validators remain active
- Network has fewer than quorum requirement (2 < 3)

**Expected behavior:**
- ✗ Consensus CANNOT continue
- ✓ No transactions are committed (safety maintained)
- ✓ System does not violate consistency
- ✓ Demonstrates Byzantine threshold enforcement

**Why it fails safely:**
With only 2 validators, the network cannot form a quorum of 3. This is **correct behavior** - the system prioritizes safety over liveness. It's better to halt than to risk committing inconsistent state.

### Scenario 4: Full Network Recovery

**What happens:**
- Both partitioned nodes (node3 and node4) are restarted
- All four validators become active again
- Network exceeds quorum requirement (4 > 3)

**Expected behavior:**
- ✓ Both nodes successfully rejoin
- ✓ Quorum is restored
- ✓ Consensus resumes normal operation
- ✓ Transactions can be committed again

**Why it works:**
Once quorum is restored, the network automatically resumes consensus. The BFT protocol ensures all validators converge to the same state without manual intervention.

## Running the Examples

### Prerequisites

- Build the `modal` CLI:
  ```bash
  cd ../../../rust
  cargo build --package modal
  ```

### Run Individual Scripts

1. **Start the 4-validator network:**
   ```bash
   ./01-run-node1.sh &  # Terminal 1
   ./02-run-node2.sh &  # Terminal 2
   ./03-run-node3.sh &  # Terminal 3
   ./04-run-node4.sh &  # Terminal 4
   ```

2. **Check network connectivity:**
   ```bash
   ./08-check-connectivity.sh
   ```

3. **Simulate single node partition:**
   ```bash
   ./05-partition-single-node.sh <node4_pid>
   ```

4. **Recover the partitioned node:**
   ```bash
   ./07-recover-node.sh 4
   ```

5. **Simulate two-node partition:**
   ```bash
   ./06-partition-two-nodes.sh <node3_pid> <node4_pid>
   ```

### Run Automated Test Suite

Run all scenarios automatically with assertions:

```bash
./test.sh
```

The test suite will:
- Start all 4 validators
- Verify network health
- Simulate single-node partition
- Demonstrate continued consensus
- Recover the partitioned node
- Simulate two-node partition
- Show consensus halt (safety)
- Recover all nodes
- Verify full network recovery

## Key Observations

### What This Demonstrates

1. **Byzantine Tolerance**: System tolerates up to `f=1` validator failures
2. **Safety Guarantees**: No commits without quorum (prevents inconsistency)
3. **Liveness Guarantees**: Progress continues when quorum exists
4. **Automatic Recovery**: Validators rejoin and sync automatically
5. **Threshold Enforcement**: System correctly enforces BFT limits

### What This Simulates

These partition scenarios are analogous to Byzantine behaviors:

| Scenario | Byzantine Analogy |
|----------|------------------|
| Single node partition | One validator goes offline (withholding) |
| Two-node partition | Multiple validators fail (exceeds threshold) |
| Node recovery | Failed validator recovers and resyncs |
| Slow node rejoin | Validator catching up after being offline |

### Differences from True Byzantine Tests

**What's tested here:**
- Observable network-level behavior
- Quorum enforcement
- Recovery mechanisms
- State synchronization

**What's NOT tested here:**
- Equivocation (conflicting certificates from same validator)
- Sophisticated Byzantine attacks
- Internal consensus state manipulation
- Reputation system dynamics

For precise Byzantine attack simulations (equivocation, sophisticated withholding), see the Rust unit tests:
- `rust/modal-validator-consensus/tests/byzantine_equivocation_tests.rs`
- `rust/modal-validator-consensus/tests/byzantine_withholding_tests.rs`

## Understanding the Results

### Success Case (Single Node Partition)

```
Network Status:
  Total validators: 4
  Active validators: 3
  Byzantine validators: 1 (offline)
  Quorum requirement: 3
  Can commit: YES ✓
```

The network continues because `active (3) ≥ quorum (3)`.

### Failure Case (Two Node Partition)

```
Network Status:
  Total validators: 4
  Active validators: 2
  Byzantine validators: 2 (offline)
  Quorum requirement: 3
  Can commit: NO ✗
```

The network halts because `active (2) < quorum (3)`. This is **correct** - safety is preserved.

## Related Examples

- **01-ping-node**: Basic network connectivity
- **03-run-devnet3**: Running a multi-validator devnet
- **04-sync-miner-blocks**: Block synchronization between nodes
- **05-mining**: Transaction ordering and block production

## Further Reading

- [Modality Network Documentation](../../../docs/network.md)
- [Byzantine Fault Tolerance Background](https://en.wikipedia.org/wiki/Byzantine_fault_tolerance)
- [Practical Byzantine Fault Tolerance (PBFT)](http://pmg.csail.mit.edu/papers/osdi99.pdf)
- [Narwhal and Tusk: A DAG-based Mempool and Efficient BFT Consensus](https://arxiv.org/abs/2105.11827)

## Troubleshooting

### Network doesn't start

- Check that ports 10301, 10302, 10303 are available
- Ensure no previous test nodes are running: `pkill -f "modal node"`
- Clear tmp directory: `rm -rf ./tmp`

### Nodes don't reconnect after recovery

- Wait 10-15 seconds for gossip protocol to propagate
- Check node logs in `./tmp/test-logs/`
- Verify bootstrapper addresses in node configs

### Tests fail intermittently

- Network consensus is inherently asynchronous
- Increase sleep timers in test.sh if nodes need more time
- Check system resources (CPU, memory) for node processes

## Cleanup

Stop all test nodes:
```bash
pkill -f "modal node"
rm -rf ./tmp
```

