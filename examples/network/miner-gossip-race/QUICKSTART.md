# Quick Start: Reproducing Your Mining Issue

## Your Specific Problem

You're seeing this pattern in your logs:
```
[INFO] ✅ Found valid nonce 121 after 122 attempts
[WARN] ⚠️  Failed to mine block 32877 (Invalid block: Mined block was rejected by fork choice rules), will retry with updated view
[INFO] ⛏️  Correcting mining index from 32877 to 32876 after error
[INFO] ⛏️  Mining block at index 32876...
[WARN] Block 32876 already exists in chain (height: 32876), skipping mining
[INFO] ✅ Successfully mined and gossipped block 32876
[INFO] ⛏️  Mining block at index 32877...
```

## Root Cause

This happens because:
1. Your miner is mining block 32877
2. Another miner gossips their version of block 32877 first
3. Your miner finishes mining but fork choice rejects it (first-seen rule)
4. Your miner tries to correct back to 32876 (but it already exists)
5. Loop continues...

## How to Reproduce with This Test

### Quick Test (5 minutes)

```bash
cd examples/network/miner-gossip

# Run the automated test
./test.sh
```

This will:
- ✅ Set up two competing miners
- ✅ Trigger the exact same race condition
- ✅ Show the rejection pattern in logs
- ✅ Measure how often it occurs
- ✅ Verify recovery works

### Manual Observation

```bash
# Terminal 1
./01-run-miner1.sh

# Terminal 2 (wait 5 seconds, then run)
./02-run-miner2.sh
```

**Watch Terminal 2** for:
```
⚠️  Failed to mine block X (Invalid block: Mined block was rejected by fork choice rules)
⛏️  Correcting mining index from X to Y after error
Block X already exists in chain, skipping mining
```

## What the Test Proves

1. **Race condition is real** - Not a bug in your setup
2. **Fork choice is working correctly** - First-seen rule prevents double-mining
3. **Recovery works** - Miners eventually sync up
4. **But inefficient** - Computational effort is wasted

## Viewing Test Results

After running `./test.sh`, check:

```bash
# View full test log
cat tmp/test-logs/miner-gossip.log

# View miner1 log
cat tmp/test-logs/miner-gossip_miner1.log

# View miner2 log (this is where race condition shows up)
cat tmp/test-logs/miner-gossip_miner2.log | grep -A 3 "rejected by fork choice"
```

## Expected Test Output

```
▶ Running: miner-gossip
Test 1: Cleaning up previous runs...
  ✓ Tmp directory should be removed
  
Test 2: Creating miner1...
  ✓ Miner1 config should be created
  
Test 3: Creating miner2...
  ✓ Miner2 config should be created
  
Test 4: Starting miner1...
  ✓ Miner1 should start on port 10401
  ✓ Miner1 should mine genesis block
  
Test 5: Starting miner2 (race condition will occur)...
  ✓ Miner2 should start on port 10402
  
Test 6: Waiting for miners to connect...
  
Test 7: Checking for race condition...
  ✓ Race condition detected (as expected)
  
Test 8: Checking for mining recovery...
  ✓ Mining recovery detected
  
Test 9: Verifying miners continue mining...
  ✓ Miner1 should have mined at least 3 blocks
  ✓ Miner2 should have at least 3 blocks in chain
  
Test 10: Verifying chain synchronization...
  ✓ Chains are synchronized (diff: 0 blocks)
  
Test 11: Analyzing race condition statistics...
  ℹ Race condition statistics:
    - Fork choice rejections: 2
    - Mining corrections: 2
    - Block skips (wasted effort): 1

✅ Tests passed: 11/11
```

## Next Steps: Testing a Fix

Once you implement a fix (e.g., pre-mining check), re-run this test:

```bash
./test.sh
```

**Success criteria:**
- Race condition count should be 0
- No "rejected by fork choice rules" messages
- No wasted mining effort
- Both miners still sync correctly

## Performance Impact

Without fix:
- Wasted PoW computation
- Retry loops
- Log spam

With fix:
- Clean mining progression
- No wasted effort
- Faster overall mining

## Implementation Recommendations

Based on the test results, we recommend **Fix #1: Pre-Mining Check**:

```rust
// In mine_and_gossip_block(), before expensive PoW
let exists = {
    let ds = datastore.lock().await;
    MinerBlock::find_canonical_by_index(&ds, index).await?.is_some()
};

if exists {
    log::info!("Block {} already exists (received via gossip), skipping mining", index);
    return Ok(());
}
```

This is:
- ✅ Simple to implement
- ✅ Low overhead (quick DB check)
- ✅ Prevents all wasted work
- ✅ No changes to fork choice logic

## Files to Modify

To implement the fix:
1. `rust/modal-node/src/actions/miner.rs` - Add pre-mining check in `mine_and_gossip_block()`
2. Re-run this test to verify fix works
3. Run full test suite to ensure no regressions

## Clean Up

```bash
./00-clean.sh
```

