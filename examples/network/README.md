# Network Examples

This directory contains dual-purpose examples that serve as:

1. **CLI Documentation** - Demonstrating how to use the Modality CLI for various network operations
2. **Integration Tests** - Automated tests that verify network functionality

## Examples Overview

| Example | Description | Category | Features Tested |
|---------|-------------|----------|-----------------|
| [01-ping-node](./01-ping-node/) | Basic node connectivity | Quick | Node startup, ping command |
| [02-run-devnet2](./02-run-devnet2/) | Two-node development network | Normal | Multi-node setup (JS + Rust) |
| [03-run-devnet3](./03-run-devnet3/) | Three-node development network | Normal | Multi-node mesh network |
| [04-sync-miner-blocks](./04-sync-miner-blocks/) | Block synchronization | Quick | Sync modes, persistence, idempotency |
| [05-mining](./05-mining/) | Mining with difficulty adjustment | Slow | Mining, difficulty, persistence |
| [06-static-validators](./06-static-validators/) | Static validator set | Slow | Validator connections, genesis round |

## Quick Start

### As CLI Documentation

Each example directory contains numbered scripts you can run interactively to learn how the CLI works:

```bash
cd 01-ping-node

# Terminal 1: Start a node
./01-run-node1.sh

# Terminal 2: Ping the node
./02-ping-node1-from-node2.sh
```

Each directory has a `README.md` with:
- Overview of the example
- Prerequisites
- Step-by-step instructions
- CLI command reference
- Architecture diagrams
- Troubleshooting tips

### As Integration Tests

Run all integration tests:

```bash
# Run quick tests only (good for CI)
./run-tests.sh --quick

# Run all tests including slow ones
./run-tests.sh --all

# Stop on first failure
./run-tests.sh --stop-on-failure
```

Run a specific test:

```bash
cd 04-sync-miner-blocks
./test.sh
```

## Test Categories

Tests are categorized by execution time:

- **Quick** (~10-30 seconds): Basic functionality, good for rapid feedback
- **Normal** (~30-60 seconds): Multi-node scenarios
- **Slow** (~1-5 minutes): Mining, long-running processes

## Test Framework

### Test Library (`test-lib.sh`)

The test framework provides utilities for:

**Process Management:**
- `test_start_process` - Start and track background processes
- `test_cleanup` - Automatically kill tracked processes
- `test_wait_for_port` - Wait for a service to be ready
- `test_wait_for_log` - Wait for a log message

**Assertions:**
- `assert_success` - Command should succeed
- `assert_failure` - Command should fail
- `assert_output_contains` - Output should contain pattern
- `assert_file_exists` - File/directory should exist
- `assert_number` - Numeric comparison

**Test Management:**
- `test_init` - Initialize test environment
- `test_finalize` - Clean up and report results
- `test_summary` - Print overall summary

### Writing a Test

Each example directory can have a `test.sh` file:

```bash
#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

# Source test library
source ../test-lib.sh

# Initialize test
test_init "my-example"

# Test 1: Start a service
echo ""
echo "Test 1: Starting service..."
PID=$(test_start_process "./start-service.sh" "service")
assert_success "test_wait_for_port 8080" "Service should start"

# Test 2: Verify functionality
echo ""
echo "Test 2: Testing functionality..."
assert_output_contains \
    "curl http://localhost:8080/status" \
    "healthy" \
    "Service should be healthy"

# Finalize (cleanup happens automatically)
test_finalize
exit $?
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Network Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run Quick Tests
        working-directory: examples/network
        run: ./run-tests.sh --quick
      
      - name: Upload Test Logs
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs
          path: examples/network/test-logs/
```

### GitLab CI

```yaml
test:network-quick:
  stage: test
  script:
    - cd examples/network
    - ./run-tests.sh --quick
  artifacts:
    when: on_failure
    paths:
      - examples/network/test-logs/

test:network-full:
  stage: test
  script:
    - cd examples/network
    - ./run-tests.sh --all
  only:
    - main
    - develop
  artifacts:
    when: on_failure
    paths:
      - examples/network/test-logs/
```

## Directory Structure

```
examples/network/
├── test-lib.sh              # Test framework library
├── run-tests.sh             # Main test runner
├── README.md                # This file
│
├── 01-ping-node/
│   ├── README.md            # Documentation
│   ├── test.sh              # Integration test
│   ├── 01-run-node1.sh      # Interactive script
│   ├── 02-ping-node1-from-node2.sh
│   └── configs/             # Node configurations
│
├── 04-sync-miner-blocks/
│   ├── README.md
│   ├── test.sh
│   ├── 00-setup-node1-blocks.sh
│   ├── 01-run-node1.sh
│   ├── 03-sync-all-blocks.sh
│   ├── 04-sync-epoch.sh
│   ├── 05-sync-range.sh
│   └── configs/
│
└── ...
```

## Best Practices

### For Documentation (Interactive Use)

1. **Numbered scripts**: Use `01-`, `02-`, etc. for sequential steps
2. **Descriptive names**: Script names should explain what they do
3. **Clear output**: Add echo statements to explain what's happening
4. **README first**: Always include a comprehensive README.md
5. **Self-contained**: Examples should work without external dependencies

### For Integration Tests

1. **Fast tests**: Keep tests under 5 minutes when possible
2. **Cleanup**: Always clean up processes and temporary files
3. **Assertions**: Use specific assertions with clear messages
4. **Logging**: Log everything to help debug failures
5. **Idempotent**: Tests should be runnable multiple times
6. **Isolated**: Tests shouldn't depend on each other

## Troubleshooting

### Tests Hanging

If a test hangs, check:
- Ports already in use (kill existing processes)
- Firewall blocking connections
- Insufficient resources (CPU/memory)

Kill all background processes:
```bash
pkill -f "modal node"
pkill -f "modality-js"
```

### Tests Failing

1. Check the test logs:
```bash
ls -la test-logs/
cat test-logs/my-example.log
```

2. Run the example manually to reproduce:
```bash
cd 01-ping-node
./01-run-node1.sh  # In one terminal
./02-ping-node1-from-node2.sh  # In another terminal
```

3. Enable debug logging:
```bash
export RUST_LOG=debug
./test.sh
```

### Port Conflicts

Each example uses specific ports:
- 01-ping-node: 10101
- 02-run-devnet2: 10101-10102
- 03-run-devnet3: 10301-10303
- 04-sync-miner-blocks: 10201-10202
- 05-mining: 10301
- 06-static-validators: 10601-10603

Check for conflicts:
```bash
lsof -i :10101
```

## Development

### Adding a New Example

1. Create a directory with a descriptive name:
```bash
mkdir 07-my-new-example
cd 07-my-new-example
```

2. Create interactive scripts:
```bash
touch 01-do-something.sh 02-do-something-else.sh
chmod +x *.sh
```

3. Write a comprehensive README.md

4. Create a test.sh:
```bash
cat > test.sh << 'EOF'
#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"
source ../test-lib.sh
test_init "07-my-new-example"
# ... add tests ...
test_finalize
exit $?
EOF
chmod +x test.sh
```

5. Add to run-tests.sh:
```bash
# In run-tests.sh
run_test_suite "07-my-new-example" "My New Example" "normal"
```

### Testing the Test Framework

Test the framework itself:

```bash
# Run quick tests only
./run-tests.sh --quick

# Run with stop on failure
./run-tests.sh --quick --stop-on-failure

# Set custom log directory
LOG_DIR=/tmp/test-logs ./run-tests.sh --quick
```

## Related Documentation

- [CLI Documentation](../../rust/modality/docs/)
- [Network Node Documentation](../../rust/modal-node/docs/)
- [Mining Documentation](../../rust/modal-miner/README.md)
- [Validator Documentation](../../rust/modal-validator/README.md)

## Contributing

When contributing new examples:

1. Ensure they serve both purposes (documentation + tests)
2. Include comprehensive README with examples
3. Write integration tests with clear assertions
4. Test locally before submitting PR
5. Update this main README with your example

## License

See [LICENSE](../../LICENSE) in the repository root.

