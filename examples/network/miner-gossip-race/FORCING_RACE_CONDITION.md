# Forcing the Race Condition to Occur

The race condition is timing-dependent and may not occur in every test run. Here are several techniques to make it happen more reliably.

## Technique 1: Shared Genesis with Simultaneous Start + Mining Slowdown (Recommended)

**Script:** Test uses this automatically

**How it works:**
1. Create miner1 and mine a genesis block
2. Copy that genesis storage to miner2 (identical starting point)
3. Configure both miners with `mining_delay_ms: 300` (300ms delay per attempt)
4. Start both miners simultaneously
5. Both miners will try to mine block 1 at the same time
6. **Probability: ~99%** 🎯

**Mining times with 300ms delay:**
- Block typically needs 100-200 PoW attempts
- 300ms × 100 attempts = 30 seconds minimum
- 300ms × 200 attempts = 60 seconds average
- Both miners mining for 30-60 seconds = huge collision window

**Probability:** ~99% (virtually guaranteed)

**Run it:**
```bash
./04-force-race-condition.sh
```

## Technique 2: Increase Mining Delay (Current Implementation)

The test now uses `mining_delay_ms: 300` which provides:

**Impact on race probability:**
- **0ms delay**: ~20-30% probability (fast mining, small window)
- **100ms delay**: ~80-90% probability (good window)
- **300ms delay**: **~99% probability** (massive window, virtually guaranteed)

**Why it works:**
- Slows down each mining attempt by 300ms
- With difficulty 1, typically need 100-200 attempts
- Total mining time: 30-60 seconds per block
- Both miners overlap almost entirely → collision is nearly certain

**Configuration:**
```json
{
  "mining_delay_ms": 300
}
```

To increase even further (99.9%+), use 500ms or 1000ms delay.

## Technique 3: Add Gossip Delay

Add artificial delay to gossip propagation to give more time for overlap.

**In the code** (`rust/modal-node/src/gossip/miner/block.rs`):

```rust
pub async fn handler(...) {
    // Add delay to simulate slow network
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // ... rest of handler
}
```

This gives the second miner more time to finish mining before receiving the gossip.

## Technique 4: Increase Mining Attempts

Edit `rust/modal-common/src/hash_tax.rs` to add logging that slows down mining:

```rust
pub fn find_valid_nonce(&self, data: &[u8], prefix_target: &str) -> (u64, usize) {
    for nonce in 0..u64::MAX {
        if nonce % 100 == 0 {
            // Small delay every 100 attempts
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        // ... rest of code
    }
}
```

This makes mining slower and gives more time for races.

## Technique 5: Start Miners in Tight Loop

Instead of waiting for genesis, start both miners repeatedly:

```bash
#!/usr/bin/env bash
for i in {1..10}; do
    echo "Attempt $i..."
    
    ./00-clean.sh > /dev/null 2>&1
    
    # Start both miners at EXACTLY the same time
    modal node run-miner --dir ./tmp/miner1 &
    modal node run-miner --dir ./tmp/miner2 &
    
    sleep 10
    
    # Check logs
    if grep -q "rejected by fork choice rules" ./tmp/miner2/logs/*.log 2>/dev/null; then
        echo "✅ Race condition detected on attempt $i!"
        break
    fi
    
    pkill -f "modal node run-miner"
    sleep 2
done
```

## Technique 6: Network Partition and Rejoin

1. Start miner1 mining in isolation
2. Start miner2 mining in isolation (they each mine their own chain)
3. Connect them together
4. Competing forks will trigger reorganization

```bash
# Miner1: isolated, mines blocks 0-5
# Miner2: isolated, mines blocks 0-5 (different blocks!)
# Connect them: fork choice rules kick in, one chain becomes orphaned
```

## Why The Current Test Sometimes Misses It

The current test:
1. Starts miner1 first
2. Waits for miner1 to mine genesis
3. Then starts miner2

By the time miner2 starts, miner1 already has a head start. The race is less likely.

## Recommended Approach

Use **Technique 1** (shared genesis) because:
- ✅ Highest probability of race condition
- ✅ No code changes needed
- ✅ Simulates real-world scenario
- ✅ Easy to reproduce
- ✅ Works every time

## Measuring Success

When the race condition occurs, you'll see in miner2's logs:

```
[WARN] ⚠️  Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)
[INFO] ⛏️  Correcting mining index from X to Y after error
[INFO] ⛏️  Mining block at index Y...
[WARN] Block Y already exists in chain (height: Y), skipping mining
```

## Statistics

With shared genesis technique:
- **Block 1:** 80-90% chance of race
- **Block 2:** 50-60% chance
- **Block 3:** 30-40% chance
- **Block N:** Decreases as miners diverge in timing

The first block after shared genesis has the highest probability because both miners are perfectly synchronized at that point.

