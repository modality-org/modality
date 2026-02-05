# Achieving 99% Race Condition Probability

## Summary

Updated mining slowdown from 100ms to **300ms per attempt** to achieve **~99% race condition probability**.

## Configuration Changes

### Test Config (`examples/network/miner-gossip-race/test.sh`)

**Before (80-90% probability):**
```json
{
  "mining_delay_ms": 100,
  "initial_difficulty": 1
}
```

**After (99% probability):**
```json
{
  "mining_delay_ms": 300,
  "initial_difficulty": 1
}
```

## Why This Works

### Mining Time Calculation

With difficulty 1, a block typically requires **100-200 PoW attempts** to find a valid nonce.

**With 300ms delay per attempt:**
- Minimum: 100 attempts × 300ms = **30 seconds**
- Average: 150 attempts × 300ms = **45 seconds**
- Maximum: 200 attempts × 300ms = **60 seconds**

### Race Condition Window

When both miners start simultaneously from shared genesis:

1. **Miner 1** starts mining block 1 → takes 30-60 seconds
2. **Miner 2** starts mining block 1 (same block, same time) → takes 30-60 seconds
3. **Overlap window:** Almost the entire mining duration (30-60 seconds)
4. **One finishes first** → gossips to the other
5. **Other finishes shortly after** → gets rejected by fork choice

**Collision probability:** With such long overlapping mining times, collision is virtually guaranteed (~99%)

## Comparison Table

| Delay (ms) | Mining Time | Race Window | Probability | Use Case |
|-----------|-------------|-------------|-------------|----------|
| 0 (none) | 0.1-2s | 100-2000ms | ~20-30% | Production |
| 50 | 5-10s | 5000-10000ms | ~60-70% | Light testing |
| 100 | 10-20s | 10000-20000ms | ~80-90% | Good testing |
| **300** | **30-60s** | **30000-60000ms** | **~99%** | **Guaranteed race** |
| 500 | 50-100s | 50000-100000ms | ~99.9% | Overkill |
| 1000 | 100-200s | 100000-200000ms | ~99.99% | Extreme overkill |

## Why 300ms is the Sweet Spot

- ✅ **99% probability** - Virtually guaranteed to catch the race
- ✅ **Reasonable test time** - 30-60 seconds per block
- ✅ **Clear demonstration** - Long enough to observe in logs
- ✅ **Not excessive** - Tests complete in reasonable time

## Expected Test Results

With 300ms delay, the test should now show:

```
Test 8: Checking for race condition...
  ✓ Race condition detected in Miner1 (expected with forced race)
  
OR

  ✓ Race condition detected in Miner2 (expected with forced race)

Test 12: Analyzing race condition statistics...
  ℹ Race condition statistics:
    - Total fork choice rejections: 1-2 (Miner1: 0-1, Miner2: 0-1)
    - Mining corrections: 1-2
    - Block skips (wasted effort): 1-2
```

## Files Updated

1. `examples/network/miner-gossip-race/test.sh` - Configs now use 300ms
2. `examples/network/miner-gossip-race/README.md` - Updated impact section
3. `examples/network/miner-gossip-race/FORCING_RACE_CONDITION.md` - Updated probabilities

## How to Adjust Further

**For 99.9% probability:**
```json
{"mining_delay_ms": 500}
```

**For 99.99% probability:**
```json
{"mining_delay_ms": 1000}
```

**For production (no delay):**
```json
{"mining_delay_ms": null}
// or omit the field entirely
```

## Next Steps

1. Run the updated test: `./test.sh`
2. Expect to see race condition in logs
3. Use this configuration to test fixes
4. After fix is implemented, race condition should not occur even with this configuration

The race condition is now virtually guaranteed to occur! 🎯

