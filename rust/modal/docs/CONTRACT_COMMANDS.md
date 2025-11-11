# Contract Commands Rearchitecture

## Overview

The contract commands have been rearchitected to follow a local-first, git-like workflow. Contracts are now stored in local directories and synced to/from validator nodes using a dedicated protocol.

## Architecture

### Local Contract Directory Structure

```
my-contract/
  .contract/
    config.json          # Contract metadata (id, remotes)
    genesis.json         # Genesis commit
    commits/             # Local commits by id
      <commit-id>.json
    HEAD                 # Current commit pointer
    refs/
      remotes/
        origin/HEAD      # Remote HEAD pointer
```

### Contract Store Module

Location: `rust/modal/src/contract_store/`

The `ContractStore` struct manages all local contract operations:
- Reading/writing directory structure
- Commit storage and retrieval
- HEAD pointer management
- Config and remote tracking

## Commands

### `modal contract create [PATH]`

Creates a new contract in a directory.

```bash
# Create contract in current directory
modal contract create

# Create contract in specific directory
modal contract create ./my-contract
```

**What it does:**
1. Generates a keypair for the contract
2. Creates `.contract/` directory structure
3. Writes genesis commit
4. Sets initial HEAD

**Output:**
- Contract ID (derived from public key)
- Genesis commit ID
- Directory location

### `modal contract commit`

Adds a commit to the local contract.

```bash
# Add a post action
modal contract commit --path /data --value "hello" --method post

# From a different directory
modal contract commit --dir ./my-contract --path /rate --value 7.5
```

**What it does:**
1. Reads current HEAD
2. Creates new commit with parent pointer
3. Computes commit ID (SHA256 hash)
4. Writes commit to `.contract/commits/`
5. Updates HEAD

### `modal contract push`

Pushes local commits to chain validators.

```bash
# Push to a validator node
modal contract push --remote /ip4/127.0.0.1/tcp/10101/p2p/12D3...

# Specify remote name
modal contract push --remote-name origin --remote <multiaddr>

# With node identity
modal contract push --remote <multiaddr> --node-dir ./node1
```

**What it does:**
1. Compares local HEAD with remote HEAD
2. Collects unpushed commits
3. Sends commits via `/contract/push` reqres protocol
4. Updates remote HEAD reference
5. Saves remote config if new

### `modal contract pull`

Pulls commits from the chain.

```bash
# Pull from configured remote
modal contract pull

# Pull from specific node
modal contract pull --remote /ip4/127.0.0.1/tcp/10101/p2p/12D3...

# Specify remote name
modal contract pull --remote-name origin
```

**What it does:**
1. Queries validator node via `/contract/pull` reqres
2. Downloads missing commits
3. Writes commits to `.contract/commits/`
4. Updates remote and local HEAD

### `modal contract status`

Shows contract status.

```bash
# Show status
modal contract status

# Status for specific directory
modal contract status --dir ./my-contract

# Compare with specific remote
modal contract status --remote origin
```

**Output:**
- Contract ID and directory
- Local HEAD commit
- Remote HEAD commit  
- Unpushed commits count
- Status (up-to-date / ahead / behind)

### `modal contract get`

(Legacy command - still available for backwards compatibility)

Gets contract or commit information from a node's datastore.

## Node-Side Protocol

### Reqres Handlers

Location: `rust/modal-node/src/reqres/contract/`

#### `/contract/push`

Accepts a batch of commits and stores them in the node's datastore.

**Request:**
```json
{
  "contract_id": "...",
  "commits": [
    {
      "commit_id": "...",
      "body": [...],
      "head": {...}
    }
  ]
}
```

**Response:**
```json
{
  "contract_id": "...",
  "pushed_count": 3,
  "status": "stored"
}
```

**Validation:**
- Verifies commit IDs match SHA256 hashes
- Checks for duplicates
- Validates commit chain integrity

#### `/contract/pull`

Returns commits for a contract, optionally after a specific commit.

**Request:**
```json
{
  "contract_id": "...",
  "since_commit_id": "..." // optional
}
```

**Response:**
```json
{
  "contract_id": "...",
  "commits": [
    {
      "commit_id": "...",
      "body": [...],
      "head": {...},
      "timestamp": 1234567890
    }
  ]
}
```

#### `/contract/list`

Lists all commits for a contract with metadata.

**Request:**
```json
{
  "contract_id": "..."
}
```

**Response:**
```json
{
  "contract_id": "...",
  "commits": [
    {
      "commit_id": "...",
      "timestamp": 1234567890,
      "in_batch": "..." // null if not yet in batch
    }
  ]
}
```

## Key Design Decisions

1. **Contract ID**: Generated from keypair public key (base58-encoded)
2. **Commit ID**: SHA256 hash of commit JSON (deterministic)
3. **Storage Format**: JSON files for human readability
4. **Sync Protocol**: RESTful-style reqres over libp2p
5. **Node Integration**: Node datastore acts as "bare repo" for contracts

## Example Workflow

```bash
# 1. Create a new contract
modal contract create ./my-contract
cd ./my-contract

# 2. Make local commits
modal contract commit --path /data --value "initial data"
modal contract commit --path /rules --value "always valid"

# 3. Check status
modal contract status

# 4. Push to validator node
modal contract push --remote /ip4/127.0.0.1/tcp/10101/p2p/12D3...

# 5. Later, pull updates from chain
modal contract pull

# 6. Check status again
modal contract status
```

## Migration from Old Commands

The old `modal contract create` and `modal contract commit` commands worked differently:

**Old behavior:**
- Created contracts directly in node datastore
- Commits submitted immediately to nodes/validators
- No local storage or version control

**New behavior:**
- Contracts stored locally first
- Commits staged locally before push
- Explicit push/pull for sync
- Git-like workflow with remotes

**Backwards compatibility:**
- The `/contract/submit` endpoint still works
- Old `modal contract get` command remains available
- Node datastore structure unchanged

## Future Enhancements

1. **Conflict Resolution**: Handle divergent commit histories
2. **Merge Commits**: Support merging parallel commit chains
3. **Branching**: Allow multiple HEAD pointers
4. **Signing**: Cryptographic signatures for commits
5. **Validation**: Contract-specific validation rules
6. **Consensus Integration**: Automatic consensus submission for pushed commits

