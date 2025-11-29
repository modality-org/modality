# Ctrl-C Signal Handling Fix

**Date**: November 29, 2025  
**Issue**: Ctrl-C (SIGINT) was not stopping miner nodes  
**Status**: ✅ **Fixed and Tested**

## Problem Summary

Users reported that pressing Ctrl-C in a tmux session did not stop miner nodes. The nodes would continue running indefinitely despite receiving the signal.

## Root Causes

Investigation revealed **three separate issues**:

### 1. Duplicate Ctrl-C Handler in Miner Mode
**Location**: `rust/modal-node/src/actions/miner.rs:35-45`

The miner action was registering its own `tokio::signal::ctrl_c()` handler. Since Tokio's signal handler can only be called once per process, this conflicted with the handler in `node.wait_for_shutdown()`.

**Fix**: Removed the duplicate handler. Only `node.wait_for_shutdown()` should register the Ctrl-C handler.

### 2. Early Shutdown Call in start_networking()
**Location**: `rust/modal-node/src/node.rs:694`

The `start_networking()` method was calling `self.shutdown()` immediately after spawning the networking task, causing premature shutdown of networking components and potentially interfering with signal handling.

**Fix**: Removed the errant `self.shutdown()` call. The method now just spawns the task and returns.

### 3. Conflicting Ctrl-C Handler in hash_tax Module  
**Location**: `rust/modal-common/src/hash_tax.rs:52-54`

The most critical issue: The `hash_tax` module (mining algorithm library) was registering its own Ctrl-C handler using `ctrlc::set_handler()`. This non-async signal handler was consuming the SIGINT signal before Tokio's `tokio::signal::ctrl_c()` could receive it.

**Fix**: 
- Removed the `ctrlc::set_handler()` registration from `hash_tax.rs`
- Added public functions `set_mining_shutdown()` and `get_mining_shutdown_flag()` to control the mining shutdown flag externally
- Updated `node.wait_for_shutdown()` to call `modal_common::hash_tax::set_mining_shutdown(true)` when Ctrl-C is received
- Added shutdown flag check in the mining error handler to exit immediately when shutdown is signaled

## Files Modified

1. **rust/modal-node/src/actions/miner.rs**
   - Removed duplicate Ctrl-C handler (lines 35-45)
   - Added shutdown check in mining error handler (line 469-472)

2. **rust/modal-node/src/node.rs**
   - Removed errant `self.shutdown()` call from `start_networking()` (line 694)
   - Added call to `modal_common::hash_tax::set_mining_shutdown(true)` in Ctrl-C handler (line 520)

3. **rust/modal-common/src/hash_tax.rs**
   - Removed `ctrlc::set_handler()` registration (lines 44-60)
   - Simplified `MINING_SHOULD_STOP` to a simple `Arc<AtomicBool>`
   - Added public `set_mining_shutdown(bool)` function
   - Added public `get_mining_shutdown_flag()` function

## Signal Flow (After Fix)

1. User presses Ctrl-C
2. `tokio::signal::ctrl_c()` receives SIGINT in `node.wait_for_shutdown()` (node.rs:518)
3. Signal handler sets flags:
   - Calls `modal_common::hash_tax::set_mining_shutdown(true)`
   - Sets `node.mining_shutdown` flag (for miner loop)
   - Broadcasts shutdown via `shutdown_tx`
4. Mining operation checks flag and stops (hash_tax.rs:220)
5. Mining error handler checks flag and exits loop (miner.rs:469)
6. Networking task receives shutdown and stops (node.rs:611)
7. Node completes graceful shutdown

## Testing

Tested with local miner node:
- Created test miner node
- Started mining
- Sent SIGINT signal
- Verified graceful shutdown within 3 seconds
- Confirmed all tasks shut down properly

**Test Result**: ✅ SUCCESS - Node stops gracefully on Ctrl-C

## Deployment

The fix has been compiled and is ready for deployment:
```bash
# Binary location
./rust/target/debug/modal

# To deploy to testnet
scp ./rust/target/debug/modal testnet1:/path/to/modal
```

## Key Learnings

1. **Only one signal handler per signal**: Mixing `ctrlc::set_handler()` with `tokio::signal::ctrl_c()` causes conflicts
2. **Signal handlers in libraries are problematic**: Library code (like `hash_tax`) should not register signal handlers - this should be done at the application level
3. **Check shutdown flags in error handlers**: Error recovery loops need to check shutdown flags to avoid infinite retry loops during shutdown
4. **Test with actual signals**: Testing with `kill -INT` (SIGINT) is essential for verifying Ctrl-C behavior

## Related Issues

- Node management commands work correctly (`modal node kill`, etc.)
- Validator and observer modes were not affected (they don't use the hash_tax mining functions)
- Only miner mode was experiencing the issue

