# Orphan Detection Test

This test validates the blockchain's orphaning logic by simulating three distinct scenarios:

## ⚠️ Note: Now Available as CLI Command

This test has been integrated into the `modal` CLI as the `modal chain validate` command. 

**Recommended usage:**

```bash
# Run all validation tests
modal chain validate

# Run specific tests
modal chain validate --test fork --test gap

# Test against existing node datastore
modal chain validate --datastore ./tmp/miner1/storage

# Get JSON output
modal chain validate --json
```

The standalone test binary in this directory is still available for reference and development purposes.

---

## Test Scenarios

### 1. Fork Detection (Single-Block Fork)
When two blocks are mined at the same index but with different content:
- Block A arrives first → accepted as canonical
- Block B arrives second → orphaned with reason: "Fork detected: block at index N has hash X, but this block expects parent hash Y"

### 2. Gap Detection
When a block references a parent that exists in the canonical chain but at the wrong index:
- Block at index N exists
- Block at index N+1 is missing (gap)
- Block at index N+2 arrives → orphaned with reason: "Gap detected: missing block(s) between index N and N+2"

### 3. Missing Parent
When a block references a parent hash that doesn't exist anywhere in the canonical chain:
- Block references unknown parent hash
- Orphaned with reason: "Parent not found: block references parent hash X which is not in the canonical chain"

## How It Works

The test uses the `modal-observer` crate's `ChainObserver` to directly test fork choice logic:

1. Creates an in-memory datastore
2. Manually constructs blocks with specific properties
3. Processes them through `ChainObserver::process_gossiped_block`
4. Verifies orphan reasons and canonical status

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

