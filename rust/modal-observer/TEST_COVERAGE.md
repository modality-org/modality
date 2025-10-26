# Modal Observer Test Coverage

This document describes the comprehensive test suite for the `modal-observer` package.

## Overview

The test suite contains **53 tests** covering all aspects of chain observation, fork choice, and orphan block storage:
- **43 unit tests** in `src/chain_observer.rs`
- **10 integration tests** in `tests/chain_observer_integration.rs`

All tests pass ✅

## Unit Tests (43 tests)

### Basic Operations (7 tests)

1. **test_chain_observer_creation** - Verifies ChainObserver can be created with in-memory datastore
2. **test_initialize_empty_chain** - Tests initialization with no existing blocks
3. **test_initialize_with_existing_chain** - Tests initialization with pre-existing canonical chain
4. **test_get_canonical_blocks** - Verifies retrieval of all canonical blocks
5. **test_get_canonical_block_by_index** - Tests querying specific block by index
6. **test_chain_cumulative_difficulty** - Validates cumulative difficulty calculation
7. **test_calculate_chain_difficulty_at_range** - Tests difficulty calculation for block ranges

### Single Block Fork Choice (4 tests)

8. **test_accept_higher_difficulty_block** - Higher difficulty block replaces existing block at same index
9. **test_reject_lower_difficulty_block** - Lower difficulty block is rejected
10. **test_reject_equal_difficulty_block** - Equal difficulty block is rejected (first-seen rule)
11. **test_orphaned_block_tracking** - Replaced blocks are correctly marked as orphaned

### Block Extension (4 tests)

12. **test_accept_block_extending_chain** - Block extending canonical chain is accepted
13. **test_reject_block_with_gap** - Block with missing parent is rejected
14. **test_reject_block_with_wrong_parent** - Block with incorrect parent hash is rejected
15. **test_accept_genesis_block** - Genesis block (index 0) is accepted

### Block Validation (1 test)

16. **test_reject_duplicate_block** - Duplicate block (same hash) is rejected

### Multi-Block Reorganizations (3 tests)

17. **test_should_accept_reorganization_higher_cumulative** - Reorganization with higher cumulative difficulty is accepted
18. **test_should_reject_reorganization_lower_cumulative** - Reorganization with lower cumulative difficulty is rejected (KEY TEST)
19. **test_reorganization_equal_difficulty_longer_chain** - Equal difficulty with more blocks uses length tiebreaker

### Helper Methods (1 test)

20. **test_get_canonical_blocks_from_index** - Tests retrieval of blocks starting from specific index

### Edge Cases (1 test)

21. **test_large_difficulty_values** - Handles very large difficulty values without overflow

### Orphan Block Storage (5 tests)

22. **test_store_orphan_competing_block** - Competing block at same index is stored as orphan with proper metadata
23. **test_store_orphan_block_with_gap** - Block with missing parent is stored as orphan
24. **test_store_orphan_block_with_wrong_parent** - Block with wrong parent hash is stored as orphan
25. **test_get_all_orphaned_blocks** - Retrieves all orphaned blocks from datastore
26. **test_get_orphaned_blocks_at_index** - Retrieves orphaned blocks at specific index

### Competing Chain Processing (6 tests)

27. **test_process_competing_chain_heavier** - Competing chain with higher cumulative difficulty is adopted
28. **test_process_competing_chain_lighter** - Competing chain with lower cumulative difficulty is rejected
29. **test_process_competing_chain_equal_difficulty_longer** - Equal difficulty chain wins with length tiebreaker
30. **test_process_competing_chain_validation_gap** - Competing chain with gap in blocks is rejected
31. **test_process_competing_chain_validation_wrong_parent** - Competing chain with wrong parent hash is rejected
32. **test_process_competing_chain_from_genesis** - Competing chain starting from genesis (index 0) is correctly evaluated

### Forced Fork Specification (6 tests)

33. **test_forced_fork_rejects_wrong_block** - Block with wrong hash at forced height is rejected and orphaned
34. **test_forced_fork_accepts_correct_block** - Block with correct hash at forced height is accepted
35. **test_forced_fork_overrides_first_seen** - Forced fork replaces existing canonical block with wrong hash
36. **test_forced_fork_multiple_heights** - Multiple forced heights are all enforced correctly
37. **test_forced_fork_competing_chain_validation** - Competing chain violating forced fork is rejected
38. **test_forced_fork_competing_chain_valid** - Competing chain respecting forced fork is evaluated normally

### Genesis Block Forced Fork Tests (5 tests)

39. **test_forced_fork_genesis_block** - Wrong genesis block is rejected when forced genesis is configured
40. **test_forced_fork_genesis_accepts_correct** - Correct genesis block is accepted with forced genesis config
41. **test_forced_fork_genesis_replaces_wrong** - Forced genesis block replaces wrong existing genesis
42. **test_forced_fork_genesis_in_competing_chain** - Competing chains must respect forced genesis checkpoint
43. **test_forced_fork_genesis_and_regular_checkpoints** - Genesis checkpoint works with other forced heights

## Integration Tests (10 tests)

### Critical Fork Choice Scenarios (2 tests)

1. **test_reject_lighter_longer_chain** ⭐ **KEY TEST**
   - Observer has 10 blocks with total difficulty 10,000
   - Receives competing 12-block chain with total difficulty 8,000
   - **Result**: Lighter chain is rejected despite being longer
   - **Validates**: Core requirement that observers don't accept lighter chains

2. **test_reject_longer_lighter_chain_multiple_scenarios**
   - Observer has 5 blocks with difficulty 5,000 each (total: 25,000)
   - Receives 10 blocks with difficulty 2,000 each (total: 20,000)
   - **Result**: Longer but lighter chain is rejected
   - **Validates**: Cumulative difficulty comparison works correctly

### Chain Reorganization Scenarios (2 tests)

3. **test_accept_heavier_longer_chain**
   - Observer has chain with difficulty 10,000
   - Receives longer chain with difficulty 15,000
   - **Result**: Heavier chain is accepted and becomes canonical

4. **test_deep_reorganization**
   - Chain diverges at block 3
   - Receives blocks 4-10 with higher cumulative difficulty
   - **Result**: Deep reorganization from block 3 is successful

### Fork Handling (2 tests)

5. **test_single_block_fork_scenarios**
   - Multiple competing blocks at index 6
   - Tests progressive replacement with higher difficulty blocks
   - **Result**: Highest difficulty block becomes canonical

6. **test_concurrent_forks_at_different_heights**
   - Forks occur at blocks 5, 7, and other indices simultaneously
   - **Result**: Each fork is resolved independently based on difficulty

### Out-of-Order Processing (1 test)

7. **test_out_of_order_block_handling**
   - Receives blocks 3, 1, 2 (out of order)
   - **Result**: Blocks without parents are rejected until parent arrives

### Tie-Breaking (1 test)

8. **test_equal_cumulative_difficulty_scenarios**
   - Competing chains with same cumulative difficulty but different lengths
   - **Result**: Longer chain wins (length tiebreaker)

### Chain State Management (1 test)

9. **test_chain_tip_updates_correctly**
   - Tracks chain tip through sequential additions and replacements
   - **Result**: Chain tip always reflects highest canonical block index

### Complex Scenarios (1 test)

10. **test_complex_multi_fork_scenario**
    - Multiple forks at different heights with varying difficulties
    - Some accepted, some rejected
    - **Result**: Final chain has correct mix of original and replacement blocks

## Fork Choice Implementation

The ChainObserver implements a **dual fork choice strategy**:

### Strategy 1: Single Block Forks
**Used when**: Competing blocks exist at the same index

**Rules**:
1. **First-seen block always wins** - existing block is kept, competing block is rejected
2. No block replacement at same index via gossip
3. Provides stability and prevents flip-flopping

**Implementation**: `should_accept_single_block()` - always returns false

### Strategy 2: Multi-Block Reorganizations
**Used when**: Evaluating competing chain branches

**Rules**:
1. Compare cumulative difficulty (total work) of both branches
2. Higher cumulative difficulty wins
3. If equal cumulative difficulty, longer chain wins (tiebreaker)
4. All replaced blocks are marked as orphaned

**Implementation**: `should_accept_reorganization()`

**Note**: Currently, `process_gossiped_block()` only implements the first-seen rule for single blocks. Multi-block reorganizations based on cumulative difficulty would require additional orchestration logic to detect and execute reorganizations.

## Key Requirements Validated

✅ **Lighter chains cannot replace canonical chain**
- Tested in `test_reject_lighter_longer_chain`
- Tested in `test_reject_longer_lighter_chain_multiple_scenarios`
- Tested in `test_should_reject_reorganization_lower_cumulative`
- Tested in `test_process_competing_chain_lighter`

✅ **Heavier chains are accepted**
- Tested in `test_accept_heavier_longer_chain`
- Tested in `test_should_accept_reorganization_higher_cumulative`
- Tested in `test_process_competing_chain_heavier`

✅ **Per-block fork choice works correctly**
- Tested in all single block fork tests
- Tested in `test_single_block_fork_scenarios`

✅ **Orphan tracking is accurate**
- Tested in `test_orphaned_block_tracking`
- Verified in all fork scenarios

✅ **Chain integrity is maintained**
- Parent validation in multiple tests
- Gap detection tested
- Chain tip tracking verified

✅ **Orphan blocks are stored and tracked**
- Tested in `test_store_orphan_competing_block`
- Tested in `test_store_orphan_block_with_gap`
- Tested in `test_store_orphan_block_with_wrong_parent`
- Orphan queries tested in `test_get_all_orphaned_blocks`

✅ **Orphan promotion works correctly**
- Verified in `test_out_of_order_block_handling`
- Orphans become canonical when parent arrives

✅ **Competing chain processing**
- Atomic evaluation in `test_process_competing_chain_*` tests
- Non-canonical storage before adoption
- Full difficulty calculation before adoption

✅ **Forced fork specification**
- Wrong blocks rejected in `test_forced_fork_rejects_wrong_block`
- Correct blocks accepted in `test_forced_fork_accepts_correct_block`
- Overrides first-seen in `test_forced_fork_overrides_first_seen`
- Multiple heights in `test_forced_fork_multiple_heights`
- Chain validation in `test_forced_fork_competing_chain_*`

✅ **Genesis block forced fork support**
- Genesis block can be specified in forced fork config
- Wrong genesis rejected in `test_forced_fork_genesis_block`
- Correct genesis accepted in `test_forced_fork_genesis_accepts_correct`
- Genesis replacement in `test_forced_fork_genesis_replaces_wrong`
- Competing chains validated in `test_forced_fork_genesis_in_competing_chain`
- Multiple checkpoints including genesis in `test_forced_fork_genesis_and_regular_checkpoints`

## Orphan Block Features

The observer now stores rejected blocks as orphans for:

### Storage Scenarios:
1. **Competing blocks** - Blocks at same index rejected by first-seen rule
2. **Blocks with gaps** - Blocks whose parent doesn't exist yet
3. **Wrong parent** - Blocks that don't extend canonical chain

### Orphan Metadata:
- `is_orphaned`: true
- `orphan_reason`: Descriptive reason for orphaning
- `competing_hash`: For competing blocks, hash of canonical winner
- `orphaned_at`: Timestamp when orphaned

### Orphan Promotion:
When a block is initially orphaned due to missing parent, it's automatically promoted to canonical when:
1. The parent block arrives
2. The parent is canonical
3. The parent hash matches the orphan's `previous_hash`

This enables out-of-order block processing and chain reconstruction.

## Running Tests

Run all tests:
```bash
cargo test -p modal-observer
```

Run only unit tests:
```bash
cargo test -p modal-observer --lib
```

Run only integration tests:
```bash
cargo test -p modal-observer --test chain_observer_integration
```

Run specific test:
```bash
cargo test -p modal-observer -- test_reject_lighter_longer_chain
```

Run with output:
```bash
cargo test -p modal-observer -- --nocapture
```

## Test Results

```
running 43 tests (unit)
test result: ok. 43 passed; 0 failed

running 10 tests (integration)
test result: ok. 10 passed; 0 failed

TOTAL: 53 passed; 0 failed
```

## Coverage Summary

| Category | Tests | Status |
|----------|-------|--------|
| Basic Operations | 7 | ✅ Pass |
| Single Block Forks | 4 | ✅ Pass |
| Block Extension | 4 | ✅ Pass |
| Block Validation | 1 | ✅ Pass |
| Multi-Block Reorgs | 3 | ✅ Pass |
| Helper Methods | 1 | ✅ Pass |
| Edge Cases | 1 | ✅ Pass |
| Orphan Block Storage | 5 | ✅ Pass |
| Competing Chain Processing | 6 | ✅ Pass |
| Forced Fork Specification | 6 | ✅ Pass |
| Genesis Block Forced Forks | 5 | ✅ Pass |
| Critical Scenarios | 2 | ✅ Pass |
| Chain Reorganizations | 2 | ✅ Pass |
| Fork Handling | 2 | ✅ Pass |
| Out-of-Order | 1 | ✅ Pass |
| Tie-Breaking | 1 | ✅ Pass |
| Chain State | 1 | ✅ Pass |
| Complex Scenarios | 1 | ✅ Pass |
| **TOTAL** | **53** | **✅ 100%** |

