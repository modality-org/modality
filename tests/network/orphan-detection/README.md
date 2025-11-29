# Orphan Detection Test

This directory contains documentation and integration tests for the blockchain's orphaning logic.

## ⚠️ Note: Tests Now in modal-miner

The orphan detection tests are now implemented as **unit tests** in the `modal-miner` crate at:
```
rust/modal-miner/src/tests.rs
```

They are also available via the `modal` CLI command for convenience.

---

## Quick Start

### Using the Modal CLI (Recommended)

```bash
# Run all validation tests via CLI
modal chain validate

# Run specific tests
modal chain validate --test fork --test gap

# Test against existing node datastore
modal chain validate --datastore ./tmp/miner1/storage

# Get JSON output
modal chain validate --json
```

### Running Unit Tests Directly

```bash
# Run the unit tests in modal-miner
cd rust/modal-miner
cargo test --features persistence orphan_detection

# Run with output
cargo test --features persistence orphan_detection -- --nocapture
```

### Running This Example's Test Script

```bash
cd examples/network/orphan-detection
./test.sh
```

This script runs both the unit tests and the CLI tests.

---

## Test Scenarios

### 1. Fork Detection (Single-Block Fork)
When two blocks are mined at the same index but with different content:
- Block A arrives first → accepted as canonical
- Block B arrives second → orphaned with reason: "Fork detected" or "Rejected by first-seen rule"

**Unit Test:** `rust/modal-miner/src/tests.rs::test_fork_detection`

### 2. Gap Detection
When a block references a parent that exists in the canonical chain but at the wrong index:
- Block at index N exists
- Block at index N+1 is missing (gap)
- Block at index N+2 arrives → orphaned with reason: "Gap detected: missing block(s) between index N and N+2"

**Unit Test:** `rust/modal-miner/src/tests.rs::test_gap_detection`

### 3. Missing Parent
When a block references a parent hash that doesn't exist anywhere in the canonical chain:
- Block references unknown parent hash
- Orphaned with reason: "Parent not found" or detected as fork

**Unit Test:** `rust/modal-miner/src/tests.rs::test_missing_parent`

### 4. Chain Integrity
Verifies that the canonical chain remains consistent after orphaning events.

**Unit Test:** `rust/modal-miner/src/tests.rs::test_chain_integrity`

### 5. Orphan Promotion
Tests that orphaned blocks can be promoted when their missing parent arrives.

**Unit Test:** `rust/modal-miner/src/tests.rs::test_orphan_promotion`

## Implementation

The tests are implemented in three places:

1. **Unit Tests** (`rust/modal-miner/src/tests.rs`)
   - Core test logic using `ChainObserver` directly
   - Fast execution with difficulty=1
   - Part of `modal-miner` crate test suite

2. **CLI Command** (`rust/modal/src/cmds/chain/validate.rs`)
   - User-friendly command-line interface
   - Supports JSON output
   - Can test against existing node datastores

3. **Integration Test** (`examples/network/orphan-detection/test.sh`)
   - Runs both unit tests and CLI tests
   - Validates all interfaces work correctly

### How Tests Work

The tests use the `modal-observer` crate's `ChainObserver` to directly test fork choice logic:

1. Create an in-memory datastore (or use existing one for CLI)
2. Manually construct blocks with specific properties (using difficulty=1)
3. Process them through `ChainObserver::process_gossiped_block`
4. Verify orphan reasons and canonical status

## Running the Test

### Using the Modal CLI (Recommended)

```bash
# Run all tests
modal chain validate

# Run with JSON output
modal chain validate --json

# Run specific tests
modal chain validate --test fork
modal chain validate --test gap --test integrity

# Test against an existing node's datastore
modal chain validate --datastore ./path/to/node/storage
```

### Using the Standalone Binary

```bash
cd examples/network/orphan-detection
cargo test
```

Or run with verbose output:
```bash
cargo test -- --nocapture
```

## Expected Output

All three test scenarios should pass:
- ✅ Fork detection correctly identifies competing blocks at the same index
- ✅ Gap detection identifies missing blocks in the chain
- ✅ Missing parent detection identifies blocks with unknown parents

## Architecture

```
orphan-detection/
├── Cargo.toml          # Test dependencies
├── README.md           # This file
└── src/
    └── main.rs         # Test implementation
```

The test is implemented as a standalone Rust project that depends on:
- `modal-observer` (for ChainObserver)
- `modal-datastore` (for data models)
- `modal-miner` (for Block creation)
- `tokio` (for async runtime)
- `anyhow` (for error handling)

## Validation

The test validates:
1. **Acceptance**: Canonical blocks are properly accepted and stored
2. **Rejection**: Orphaned blocks are rejected and marked as non-canonical
3. **Reason Accuracy**: Orphan reasons correctly describe why a block was rejected
4. **Chain Integrity**: The canonical chain remains consistent after orphaning events

