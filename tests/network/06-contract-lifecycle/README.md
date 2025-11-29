# Example: Contract Lifecycle

This example demonstrates the complete contract lifecycle in Modality, including creating contracts, making commits, checking status, pushing to validators, and pulling from the network.

## Overview

Modality contracts use a **local-first, git-like workflow**:

- **Create**: Initialize a new contract locally with `modal contract create`
- **Commit**: Add changes to the local contract with `modal contract commit`
- **Status**: View local and remote contract state with `modal contract status`
- **Push**: Push local commits to network validators with `modal contract push`
- **Pull**: Pull remote commits from validators with `modal contract pull`

All contract data is stored in a `.contract/` directory (similar to `.git/`).

## Prerequisites

1. Build the `modal` CLI:
   ```bash
   cd ../../../rust
   cargo build --package modal
   ```

2. (Optional) For push/pull operations, you'll need a running validator node.

## Quick Start

Run the complete lifecycle demonstration:

```bash
./06-full-lifecycle.sh
```

This will:
1. Create a new contract
2. Add several commits
3. View the contract status
4. Guide you through push/pull operations

## Step-by-Step Guide

### Step 1: Create a Contract

```bash
./01-create-contract.sh
```

This creates a new contract in `./tmp/my-contract/` with the following structure:

```
./tmp/my-contract/
├── .contract/
│   ├── config.json      # Contract configuration and ID
│   ├── genesis.json     # Initial contract state
│   ├── HEAD             # Current commit pointer
│   └── commits/         # Local commits (created later)
```

**Example output:**
```
✅ Contract created successfully!

📋 Contract Details:
   Contract ID: contract_abc123...
   Directory: ./tmp/my-contract
```

### Step 2: Add Commits

```bash
./02-commit-to-contract.sh
```

This creates three commits with different data:
- Commit 1: String value at `/data/message`
- Commit 2: Numeric value at `/config/rate`
- Commit 3: Status value at `/data/status`

**Example output:**
```
✅ All commits created successfully!

📊 Commit Summary:
   Total commits: 3
   Commit 1: commit_xyz789...
   Commit 2: commit_def456...
   Commit 3: commit_ghi123...
```

### Step 3: View Status

```bash
./03-view-status.sh
```

Shows the current state of your contract:

**Example output:**
```
Contract ID: contract_abc123...
Local commits: 3
Remote commits: 0
Status: local changes not pushed
```

### Step 4: Push to Validators

```bash
./04-push-to-validators.sh
```

This script will:
1. Create a validator node (if needed)
2. Start the validator
3. Push your local commits to the network

**Example output:**
```
✅ Commits pushed successfully!

📊 Push Summary:
   Commits pushed: 3
```

**Note**: Requires a running validator node. The script automatically creates and starts one using the `devnet1/node1` template.

### Step 5: Pull from Network

```bash
./05-pull-from-network.sh
```

Demonstrates pulling commits from the network. In a real scenario, this would fetch commits made by other users or from other locations.

## Script Reference

| Script | Description |
|--------|-------------|
| `01-create-contract.sh` | Create a new contract locally |
| `02-commit-to-contract.sh` | Add three commits to the contract |
| `03-view-status.sh` | View contract status (local and remote) |
| `04-push-to-validators.sh` | Push local commits to network validators |
| `05-pull-from-network.sh` | Pull remote commits from the network |
| `06-full-lifecycle.sh` | Run the complete lifecycle demo |
| `test.sh` | Automated integration test |

## Command Reference

### `modal contract create`

Creates a new contract in the current directory.

```bash
modal contract create [OPTIONS]

Options:
  --output <FORMAT>    Output format: text or json [default: text]
```

**Creates:**
- `.contract/` directory
- `config.json` with contract ID and settings
- `genesis.json` with initial state
- `HEAD` file tracking current commit

### `modal contract commit`

Add a commit to the local contract.

```bash
modal contract commit --path <PATH> --value <VALUE> [OPTIONS]

Options:
  --path <PATH>        Path in the contract state tree (e.g., /data/key)
  --value <VALUE>      Value to set (string, number, or JSON)
  --output <FORMAT>    Output format: text or json [default: text]
```

**Examples:**
```bash
# String value
modal contract commit --path "/user/name" --value "Alice"

# Numeric value
modal contract commit --path "/config/timeout" --value 30

# JSON value
modal contract commit --path "/data/user" --value '{"name":"Alice","age":30}'
```

### `modal contract status`

Show the status of the local contract and its relationship to the network.

```bash
modal contract status [OPTIONS]

Options:
  --output <FORMAT>    Output format: text or json [default: text]
```

**Shows:**
- Contract ID
- Local commit count
- Remote commit count (if connected to network)
- Push/pull status
- Last sync time

### `modal contract push`

Push local commits to network validators.

```bash
modal contract push [OPTIONS]

Options:
  --output <FORMAT>    Output format: text or json [default: text]
  --force              Force push even if remote has diverged
```

**Behavior:**
- Pushes all unpushed local commits
- Validates commits before sending
- Updates remote tracking
- Fails if network is unreachable

### `modal contract pull`

Pull commits from network validators to local contract.

```bash
modal contract pull [OPTIONS]

Options:
  --output <FORMAT>    Output format: text or json [default: text]
  --merge              Automatically merge remote changes
```

**Behavior:**
- Fetches all remote commits not in local history
- Updates local commit history
- Optionally merges changes
- Fails if local has uncommitted changes

## Contract Directory Structure

```
.contract/
├── config.json          # Contract configuration
│   ├── contract_id      # Unique contract identifier
│   ├── network          # Network configuration
│   └── created_at       # Creation timestamp
│
├── genesis.json         # Genesis state
│   ├── initial_state    # Initial contract state
│   └── validators       # Genesis validators
│
├── HEAD                 # Current commit pointer
│
├── commits/             # Local commits
│   ├── <commit1>.json   # Individual commit files
│   ├── <commit2>.json
│   └── ...
│
└── refs/                # Remote references (created on push/pull)
    └── remote/          # Remote commit tracking
```

## Testing

Run the automated integration test:

```bash
./test.sh
```

This tests:
- ✓ Contract creation
- ✓ Directory structure validation
- ✓ Multiple commits
- ✓ Status command (text and JSON)
- ✓ Commit storage and counting
- ✓ Push to validators
- ✓ Pull from network
- ✓ Contract ID consistency
- ✓ JSON output validation

**Example output:**
```
✓ 06-contract-lifecycle passed (17/17 tests)
```

## Use Cases

### Local Development

Work on contracts locally without network connectivity:

```bash
cd my-project
modal contract create
modal contract commit --path "/test" --value "data"
modal contract status
```

### Collaborative Development

Push changes to share with others:

```bash
# Alice's machine
modal contract create
modal contract commit --path "/data" --value "Alice's data"
modal contract push

# Bob's machine (with same contract ID)
modal contract pull
modal contract status  # Shows Alice's commit
```

### Versioning and Rollback

View and manage contract history:

```bash
# View all commits
modal contract log

# View specific commit
modal contract show <commit-id>

# Revert to previous state
modal contract revert <commit-id>
```

## Architecture

```
┌─────────────────────┐
│   Local Contract    │
│                     │
│  .contract/         │
│  ├── config.json    │
│  ├── commits/       │
│  └── HEAD           │
└──────────┬──────────┘
           │
           │ push/pull
           │
           ▼
┌─────────────────────┐
│   Network Layer     │
│                     │
│  ┌───────────────┐  │
│  │  Validator 1  │  │
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  Validator 2  │  │
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  Validator 3  │  │
│  └───────────────┘  │
└─────────────────────┘
```

## Comparison with Git

Modality's contract system is inspired by Git:

| Git | Modality Contracts |
|-----|-------------------|
| `git init` | `modal contract create` |
| `git commit` | `modal contract commit` |
| `git status` | `modal contract status` |
| `git push` | `modal contract push` |
| `git pull` | `modal contract pull` |
| `.git/` directory | `.contract/` directory |
| Remote repository | Network validators |

**Key differences:**
- Contracts are stored on a decentralized network, not a central server
- Commits include state changes, not file diffs
- Validators verify and store commits, providing consensus

## Troubleshooting

### "Contract not found"

Make sure you're in a directory with a `.contract/` folder, or run `modal contract create` first.

### "Failed to push: No validators reachable"

Ensure at least one validator node is running and accessible. Use `./04-push-to-validators.sh` which automatically sets up a local validator.

### "Failed to pull: Contract ID mismatch"

The local contract ID doesn't match the remote. Ensure you're working with the correct contract.

### "Commit failed: Invalid path"

Paths must start with `/` and follow a valid format (e.g., `/data/key`, not `data/key`).

## Advanced Usage

### Working with JSON Output

All commands support `--output json` for scripting:

```bash
# Get contract ID
CONTRACT_ID=$(modal contract status --output json | jq -r '.contract_id')

# Count local commits
LOCAL_COMMITS=$(modal contract status --output json | jq '.local_commits')

# Check if push is needed
NEEDS_PUSH=$(modal contract status --output json | jq '.needs_push')
```

### Batch Commits

Create multiple commits in a script:

```bash
#!/bin/bash
for i in {1..10}; do
  modal contract commit --path "/data/item$i" --value "value$i"
done
```

### Integration with CI/CD

```yaml
# .github/workflows/contract.yml
- name: Push contract changes
  run: |
    cd my-contract
    modal contract push --output json > push_result.json
    cat push_result.json | jq .
```

## Related Documentation

- [Contract Rearchitecture Summary](../../../CONTRACT_REARCHITECTURE_SUMMARY.md)
- [Contract Implementation Complete](../../../CONTRACT_COMPLETE.md)
- [Network Examples](../README.md)

## Notes

- Contract operations are local-first - work offline and sync later
- The `.contract/` directory should be tracked in version control (like `.git/`)
- Contract IDs are deterministic based on genesis state and creator
- Push/pull operations require network connectivity and running validators
- All commits are immutable once pushed to the network

## Next Steps

After completing this example, explore:

1. **[02-run-devnet1](../02-run-devnet1/)** - Learn about running validator nodes
2. **[05-mining](../05-mining/)** - Understand mining and block creation
3. **[Network Documentation](../README.md)** - Comprehensive network guide

## Feedback

If you encounter issues or have suggestions, please open an issue or contribute to the documentation!

