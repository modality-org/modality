# Network Examples - Quick Reference

## Run Examples Interactively

```bash
# Ping example
cd 01-ping-node
./01-run-node1.sh              # Terminal 1
./02-ping-node1-from-node2.sh  # Terminal 2

# Sync example
cd 04-sync-miner-blocks
./01-run-node1.sh              # Terminal 1 (auto-creates blocks)
./03-sync-all-blocks.sh        # Terminal 2 (syncs blocks)

# Mining example
cd 05-mining
./01-mine-blocks.sh            # Mines continuously
./02-inspect-blocks.sh         # Check blocks (separate terminal)

# Validators example
cd 06-static-validators
./01-run-validator1.sh         # Terminal 1
./02-run-validator2.sh         # Terminal 2
./03-run-validator3.sh         # Terminal 3
# Or: ./06-run-all-validators.sh (runs all in background)
```

## Run as Integration Tests

```bash
# Quick tests (~30 seconds)
./run-tests.sh --quick

# All tests (~5 minutes)
./run-tests.sh --all

# Stop on first failure
./run-tests.sh --quick --stop-on-failure

# Individual test
cd 01-ping-node && ./test.sh
```

## View Test Results

```bash
# View test logs
ls -la tmp/test-logs/
cat tmp/test-logs/01-ping-node.log

# Clean up logs (they're gitignored)
rm -rf tmp/test-logs/

# Note: Log files are automatically ignored by .gitignore
# - tmp/test-logs/ directory
# - *.log files
# - */tmp/ directories
```

## CI/CD Integration

**GitHub Actions:**
```yaml
- name: Run Tests
  working-directory: examples/network
  run: ./run-tests.sh --quick
```

**GitLab CI:**
```yaml
test:
  script:
    - cd examples/network
    - ./run-tests.sh --quick
```

**Docker:**
```bash
docker build -f examples/network/Dockerfile.test -t tests .
docker run --rm tests --quick
```

## Test Categories

| Category | Examples | Duration | Use Case |
|----------|----------|----------|----------|
| Quick | 01, 04 | ~30s | CI/PR checks |
| Normal | 02, 03 | ~60s | Development |
| Slow | 05, 06 | ~5m | Full validation |

## Available Examples

| # | Name | Demonstrates |
|---|------|--------------|
| 01 | ping-node | Basic connectivity |
| 02 | run-devnet2 | Two-node network |
| 03 | run-devnet3 | Three-node network |
| 04 | sync-miner-blocks | Block sync (all modes) |
| 05 | mining | Mining & difficulty |
| 06 | static-validators | Validator setup |

## Common Commands

```bash
# Build CLI
cd rust && cargo build --package modal

# Kill all nodes
pkill -f "modal node"

# Check port usage
lsof -i :10101

# Enable debug logging
export RUST_LOG=debug
./test.sh
```

## Documentation

- **README.md** - Full documentation
- **CI-CD-GUIDE.md** - CI/CD integration
- **IMPLEMENTATION_SUMMARY.md** - Implementation details
- **Each example's README.md** - Specific example docs

## Writing Tests

```bash
#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

source ../test-lib.sh
test_init "my-test"

# Start process
PID=$(test_start_process "./my-script.sh" "service")
assert_success "test_wait_for_port 8080" "Should start"

# Test functionality
assert_output_contains \
    "curl localhost:8080" \
    "OK" \
    "Should respond"

test_finalize
exit $?
```

## Troubleshooting

**Tests hanging?**
- Kill processes: `pkill -f "modal node"`
- Check ports: `lsof -i :10101`

**Tests failing?**
- Check logs: `cat tmp/test-logs/*.log`
- Run manually: `cd 01-ping-node && ./01-run-node1.sh`
- Enable debug: `export RUST_LOG=debug`

**Port conflicts?**
- Each example uses different ports
- See example README for port numbers
- Kill conflicting processes

## Quick Validation

Test the entire system:
```bash
# 1. Build
cd rust && cargo build --package modal

# 2. Quick smoke test
cd ../examples/network
./run-tests.sh --quick

# 3. If passing, ready to commit!
```

