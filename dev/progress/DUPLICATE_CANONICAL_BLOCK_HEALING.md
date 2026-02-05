# Duplicate Canonical Block Detection and Healing Implementation

## Issue Found on testnet2

testnet2's datastore had **TWO canonical blocks at index 2032**, which caused:
- `find_all_canonical` returned 32877 blocks for indices 0-32875 (one extra due to duplicate)
- Maximum index was 32875
- Next mining index calculation returned 32876
- Node would skip mining 32876 because count mismatch made it think it existed
- Infinite loop: try to mine 32876 → skip → recalculate → 32876 again

Additionally, there were **299 orphaned blocks at index 32877** from repeated failed mining attempts.

## Root Cause

The duplicate at index 2032 violated the invariant that each index should have exactly one canonical block. This happened due to a race condition or bug in the fork choice code that allowed two blocks to both be marked canonical at the same time.

## Implementation

### 1. MinerBlockHeight Model (COMPLETE)

Created `rust/modal-datastore/src/models/miner/miner_block_height.rs`:
- Lightweight index model storing only `index`, `block_hash`, and `is_canonical`
- Indexed by `/miner_blocks/index/${index}/hash/${block_hash}`
- Provides efficient lookup of canonical blocks by height
- Methods: `find_canonical_by_index()`, `find_all_by_index()`, `delete()`

### 2. MinerBlock Integration (COMPLETE)

Updated `MinerBlock` in `rust/modal-datastore/src/models/miner/miner_block.rs`:
- Custom `save()` method that maintains both hash-based and height-based indices
- Automatically creates/updates `MinerBlockHeight` entry when saving
- Updated `save_as_pending()` and `canonize()` signatures to use `&mut NetworkDatastore`

### 3. Duplicate Detection and Healing Logic (COMPLETE)

Created `rust/modal-datastore/src/models/miner/integrity.rs`:

#### `detect_duplicate_canonical_blocks()`
- Queries all canonical blocks
- Groups by index
- Returns indices with more than one canonical block

#### `heal_duplicate_canonical_blocks()`
Healing strategy (fork choice rules):
1. **First-seen**: Keep block with earliest `seen_at` timestamp
2. **Difficulty**: If timestamps equal, keep higher difficulty
3. **Hash**: If still tied, keep lexicographically smaller hash (deterministic)

The function:
- Sorts duplicate blocks by the above rules
- Keeps first block as canonical
- Marks others as orphaned with reason "Duplicate canonical block at index {}"
- Updates both `MinerBlock` and `MinerBlockHeight` entries
- Returns list of orphaned block hashes

### 4. Tests (COMPLETE)

All tests passing in `modal-datastore`:
- `MinerBlockHeight` model tests (create, save, find canonical, detect duplicates)
- Integrity tests (detect no duplicates, detect and heal duplicates)
- Updated existing `MinerBlock` tests for new `&mut` signature

## Next Steps (TODO)

All implementation steps are now complete:
- ✅ MinerBlockHeight model created
- ✅ MinerBlock integration complete
- ✅ Duplicate detection and healing logic implemented
- ✅ Tests passing in modal-datastore
- ✅ Node startup auto-detection added
- ✅ Manual heal command created (`modal chain heal`)
- ✅ Chain validate integration complete
- ✅ Integration tests passing

## Usage

### Auto-Detection on Node Startup

Nodes now automatically detect and heal duplicate canonical blocks during startup:

```bash
modal node run -c config.json
```

If duplicates are found, you'll see log messages like:
```
⚠️  Detected 1 indices with duplicate canonical blocks
  Index 2032: 2 canonical blocks
    - hash_1a... (seen_at: Some(1000))
    - hash_1b... (seen_at: Some(1005))
🔧 Auto-healing duplicate canonical blocks...
✅ Healed 1 duplicate blocks
```

### Manual Healing

Use the `modal chain heal` command to manually detect and heal duplicates:

```bash
# Dry run (detect only, don't fix)
modal chain heal --datastore ~/.modal/storage --dry-run

# Actually heal the duplicates
modal chain heal --datastore ~/.modal/storage
```

### Validation

Check for duplicates using the validation command:

```bash
modal chain validate --test duplicate-canonical --datastore ~/.modal/storage
```

## How It Fixes testnet2's Issue

When the healing logic runs:
1. Detects two canonical blocks at index 2032
2. Compares them using fork choice rules
3. Keeps one as canonical, marks other as orphaned
4. Count becomes correct: 32876 canonical blocks (0-32875)
5. Next mining index correctly calculates as 32876
6. Mining can proceed normally

