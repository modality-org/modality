# CI/CD Integration Guide for Network Examples

This guide explains how to integrate the network examples as automated tests in your CI/CD pipeline.

## Overview

The network examples can be run as integration tests in continuous integration environments. The test framework is designed to:

- ✅ Run in containerized environments (Docker, GitHub Actions, GitLab CI)
- ✅ Support parallel execution where possible
- ✅ Provide clear exit codes and test reports
- ✅ Clean up resources automatically
- ✅ Generate logs for debugging failures

## Quick Setup

### Prerequisites

Your CI environment needs:
- Rust toolchain (stable)
- Bash shell
- Standard Unix utilities (lsof, grep, etc.)
- Network capabilities (for multi-node tests)
- ~2GB RAM minimum
- ~5GB disk space

## GitHub Actions

### Basic Setup

Create `.github/workflows/network-tests.yml`:

```yaml
name: Network Integration Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  network-tests-quick:
    name: Quick Network Tests
    runs-on: ubuntu-latest
    timeout-minutes: 10
    
    steps:
      - name: Checkout code
        uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      
      - name: Cache Rust dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            rust/target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run Quick Tests
        working-directory: examples/network
        run: |
          ./run-tests.sh --quick
      
      - name: Upload Test Logs
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs-quick
          path: examples/network/tmp/test-logs/
          retention-days: 7

  network-tests-full:
    name: Full Network Tests
    runs-on: ubuntu-latest
    timeout-minutes: 30
    # Only run on main branch to save CI time
    if: github.ref == 'refs/heads/main'
    
    steps:
      - name: Checkout code
        uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      
      - name: Cache Rust dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            rust/target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run All Tests
        working-directory: examples/network
        run: |
          ./run-tests.sh --all
      
      - name: Upload Test Logs
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs-full
          path: examples/network/tmp/test-logs/
          retention-days: 7
```

### Advanced: Matrix Strategy

Run tests across multiple Rust versions:

```yaml
jobs:
  network-tests:
    name: Network Tests (Rust ${{ matrix.rust }})
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        rust: [stable, beta]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust ${{ matrix.rust }}
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: ${{ matrix.rust }}
          override: true
      
      - name: Run Tests
        working-directory: examples/network
        run: ./run-tests.sh --quick
```

### Advanced: Separate Test Jobs

Run each example as a separate job for better parallelization:

```yaml
jobs:
  test-ping:
    name: Test - Ping Node
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build CLI
        run: cd rust && cargo build --package modal
      - name: Run Test
        working-directory: examples/network/01-ping-node
        run: ./test.sh
      - if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: logs-ping
          path: examples/network/tmp/test-logs/

  test-sync:
    name: Test - Sync Miner Blocks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build CLI
        run: cd rust && cargo build --package modal
      - name: Run Test
        working-directory: examples/network/04-sync-miner-blocks
        run: ./test.sh
      - if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: logs-sync
          path: examples/network/tmp/test-logs/
```

## GitLab CI

### Basic Setup

Create `.gitlab-ci.yml`:

```yaml
stages:
  - build
  - test-quick
  - test-full

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo
  RUST_LOG: info

cache:
  key: ${CI_COMMIT_REF_SLUG}
  paths:
    - .cargo/
    - rust/target/

build:modal:
  stage: build
  image: rust:latest
  script:
    - cd rust
    - cargo build --package modal --release
  artifacts:
    paths:
      - rust/target/release/modal
    expire_in: 1 hour

test:network:quick:
  stage: test-quick
  image: rust:latest
  dependencies:
    - build:modal
  script:
    - cd examples/network
    - ./run-tests.sh --quick
  artifacts:
    when: on_failure
    paths:
      - examples/network/tmp/test-logs/
    expire_in: 1 week

test:network:full:
  stage: test-full
  image: rust:latest
  dependencies:
    - build:modal
  only:
    - main
    - develop
  script:
    - cd examples/network
    - ./run-tests.sh --all
  artifacts:
    when: always
    paths:
      - examples/network/tmp/test-logs/
    expire_in: 1 week
  timeout: 30m
```

### With Docker

```yaml
build:modal:
  stage: build
  image: rust:1.70-slim
  before_script:
    - apt-get update
    - apt-get install -y build-essential lsof
  script:
    - cd rust
    - cargo build --package modal
  artifacts:
    paths:
      - rust/target/debug/modal
    expire_in: 1 hour

test:network:quick:
  stage: test-quick
  image: rust:1.70-slim
  dependencies:
    - build:modal
  before_script:
    - apt-get update
    - apt-get install -y lsof procps
  script:
    - cd examples/network
    - ./run-tests.sh --quick
```

## CircleCI

Create `.circleci/config.yml`:

```yaml
version: 2.1

orbs:
  rust: circleci/rust@1.6.0

jobs:
  build:
    docker:
      - image: cimg/rust:1.70
    steps:
      - checkout
      - restore_cache:
          keys:
            - cargo-cache-{{ checksum "rust/Cargo.lock" }}
      - run:
          name: Build Modal CLI
          command: cd rust && cargo build --package modal
      - save_cache:
          paths:
            - ~/.cargo
            - rust/target
          key: cargo-cache-{{ checksum "rust/Cargo.lock" }}
      - persist_to_workspace:
          root: .
          paths:
            - rust/target/debug/modal

  test-quick:
    docker:
      - image: cimg/rust:1.70
    steps:
      - checkout
      - attach_workspace:
          at: .
      - run:
          name: Run Quick Tests
          command: cd examples/network && ./run-tests.sh --quick
      - store_artifacts:
          path: examples/network/tmp/test-logs
          destination: test-logs

  test-full:
    docker:
      - image: cimg/rust:1.70
    steps:
      - checkout
      - attach_workspace:
          at: .
      - run:
          name: Run Full Tests
          command: cd examples/network && ./run-tests.sh --all
          no_output_timeout: 30m
      - store_artifacts:
          path: examples/network/tmp/test-logs
          destination: test-logs

workflows:
  version: 2
  test:
    jobs:
      - build
      - test-quick:
          requires:
            - build
      - test-full:
          requires:
            - build
          filters:
            branches:
              only:
                - main
                - develop
```

## Jenkins

Create `Jenkinsfile`:

```groovy
pipeline {
    agent any
    
    environment {
        CARGO_HOME = "${WORKSPACE}/.cargo"
        RUST_LOG = "info"
    }
    
    stages {
        stage('Setup') {
            steps {
                sh 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
            }
        }
        
        stage('Build') {
            steps {
                sh '''
                    cd rust
                    cargo build --package modal
                '''
            }
        }
        
        stage('Test - Quick') {
            steps {
                sh '''
                    cd examples/network
                    ./run-tests.sh --quick
                '''
            }
        }
        
        stage('Test - Full') {
            when {
                branch 'main'
            }
            steps {
                timeout(time: 30, unit: 'MINUTES') {
                    sh '''
                        cd examples/network
                        ./run-tests.sh --all
                    '''
                }
            }
        }
    }
    
    post {
        always {
            archiveArtifacts artifacts: 'examples/network/tmp/test-logs/**/*.log', allowEmptyArchive: true
        }
        failure {
            emailext (
                subject: "Failed: Job '${env.JOB_NAME} [${env.BUILD_NUMBER}]'",
                body: "Check logs at ${env.BUILD_URL}",
                to: 'dev-team@example.com'
            )
        }
    }
}
```

## Docker

### Dockerfile for Testing

Create `examples/network/Dockerfile.test`:

```dockerfile
FROM rust:1.70-slim

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    build-essential \
    lsof \
    procps \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace
COPY . .

# Build modal CLI
RUN cd rust && cargo build --package modal

# Set working directory to examples
WORKDIR /app/examples/network

# Run tests
ENTRYPOINT ["./run-tests.sh"]
CMD ["--all"]
```

### Usage

```bash
# Build test image
docker build -f examples/network/Dockerfile.test -t modality-network-tests .

# Run quick tests
docker run --rm modality-network-tests --quick

# Run all tests
docker run --rm modality-network-tests --all

# Run with custom log directory
docker run --rm -v $(pwd)/logs:/app/examples/network/tmp/test-logs modality-network-tests --quick
```

## Test Reports

### JUnit XML Output

Add test reporting with a wrapper script `examples/network/run-tests-junit.sh`:

```bash
#!/usr/bin/env bash
# Generate JUnit XML from test results

OUTPUT_DIR=${OUTPUT_DIR:-./test-results}
mkdir -p "$OUTPUT_DIR"

# Run tests and capture output
./run-tests.sh "$@" 2>&1 | tee test-output.txt

# Generate JUnit XML (simplified)
cat > "$OUTPUT_DIR/junit.xml" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="Network Examples" tests="6" failures="0" errors="0" skipped="0">
    <!-- Parse test-output.txt and generate test cases -->
  </testsuite>
</testsuites>
EOF

# Clean up
rm -f test-output.txt
```

### HTML Reports

Generate HTML reports using a simple script:

```bash
#!/usr/bin/env bash
# Generate HTML report from logs

cat > test-report.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Network Test Report</title>
    <style>
        body { font-family: monospace; }
        .pass { color: green; }
        .fail { color: red; }
        pre { background: #f4f4f4; padding: 10px; }
    </style>
</head>
<body>
    <h1>Network Integration Test Report</h1>
    <pre>$(cat tmp/test-logs/*.log)</pre>
</body>
</html>
EOF
```

## Best Practices

### Resource Limits

Set appropriate timeouts and resource limits:

```yaml
# GitHub Actions
jobs:
  test:
    timeout-minutes: 15
    runs-on: ubuntu-latest
```

```yaml
# GitLab CI
test:
  timeout: 15m
  resource_group: network-tests  # Prevent parallel runs
```

### Cleanup

Ensure cleanup happens even on failure:

```bash
# In your CI script
trap 'pkill -f "modal node"; exit' INT TERM EXIT
./run-tests.sh --quick
```

### Artifacts

Always upload logs on failure:

```yaml
# GitHub Actions
- name: Upload Test Logs
  if: failure()
  uses: actions/upload-artifact@v3
  with:
    name: test-logs
    path: examples/network/test-logs/
```

### Caching

Cache Rust dependencies to speed up builds:

```yaml
# GitHub Actions
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/
      rust/target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

## Troubleshooting CI Failures

### Port Conflicts

CI environments may have services using ports:

```bash
# In CI script, check for port conflicts before tests
netstat -tuln | grep -E ':(10101|10201|10301|10601)'
```

### Timeouts

Increase timeouts for slow CI runners:

```bash
# Modify test-lib.sh locally or via environment
export DEFAULT_TIMEOUT=60  # seconds
./run-tests.sh --quick
```

### Resource Constraints

Some CI runners have limited resources:

```yaml
# GitHub Actions - use larger runner
jobs:
  test:
    runs-on: ubuntu-latest-4-cores  # If available
```

### Flaky Tests

If tests are flaky:

1. Add retries:
```yaml
# GitHub Actions
- name: Run Tests
  uses: nick-invision/retry@v2
  with:
    timeout_minutes: 10
    max_attempts: 3
    command: cd examples/network && ./run-tests.sh --quick
```

2. Investigate logs
3. Increase wait times in tests
4. Run tests sequentially instead of in parallel

## Monitoring

### Set up notifications

```yaml
# GitHub Actions - Slack notification
- name: Slack Notification
  if: failure()
  uses: rtCamp/action-slack-notify@v2
  env:
    SLACK_WEBHOOK: ${{ secrets.SLACK_WEBHOOK }}
    SLACK_MESSAGE: 'Network tests failed'
```

### Track test duration

```bash
# Add to run-tests.sh
START_TIME=$(date +%s)
# ... run tests ...
END_TIME=$(date +%s)
echo "Total duration: $((END_TIME - START_TIME))s"
```

## Example: Complete GitHub Actions Workflow

```yaml
name: Network Integration Tests

on:
  push:
    branches: [main, develop]
  pull_request:
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight

jobs:
  network-tests:
    name: ${{ matrix.test-suite }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    strategy:
      fail-fast: false
      matrix:
        test-suite: [quick, full]
        include:
          - test-suite: quick
            args: --quick
            cache-key: quick
          - test-suite: full
            args: --all
            cache-key: full
    
    steps:
      - name: Checkout
        uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      
      - name: Cache
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/
            rust/target/
          key: ${{ runner.os }}-${{ matrix.cache-key }}-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run Tests
        working-directory: examples/network
        run: ./run-tests.sh ${{ matrix.args }}
      
      - name: Upload Logs
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs-${{ matrix.test-suite }}
          path: examples/network/tmp/test-logs/
```

## Summary

The network examples are designed to work seamlessly in CI/CD pipelines with:

- ✅ Clear exit codes (0 = success, 1 = failure)
- ✅ Automatic cleanup of resources
- ✅ Detailed logging for debugging
- ✅ Fast feedback with `--quick` mode
- ✅ Comprehensive testing with `--all` mode

Choose the configuration that best fits your CI platform and customize as needed!

