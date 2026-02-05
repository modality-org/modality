# CLI Reference

The `modal` command-line tool is your interface to Modality.

## Command Overview

| Command | Description |
|---------|-------------|
| `modal contract` | Contract management (create, commit, push, pull) |
| `modal id` | Identity management (create, derive, get) |
| `modal passfile` | Passfile encryption/decryption |
| `modal predicate` | Predicate management and testing |
| `modal node` | Network node operations |
| `modal status` | Show contract status (if in contract directory) |

---

## Contract Commands (`modal contract` / `modal c`)

### Create a Contract

```bash
modal contract create
modal c create
```

Creates a new contract in the current directory. Initializes `.contract/` for tracking commits.

### Checkout State

```bash
modal c checkout
```

Extracts the current state from commits to the `state/` directory for editing.

### Set State Values

```bash
# Set a value at a path
modal c set /path/to/file.txt "content"

# Set from a file
modal c set /data/config.json --file ./local-config.json

# Set an ID from a passfile
modal c set-named-id /parties/alice.id alice
```

### Commit Changes

```bash
# Commit all changes (state + rules)
modal c commit --all -m "Description"

# Commit with signature
modal c commit --all --sign alice.passfile -m "Signed commit"

# Commit only state changes
modal c commit --state -m "State update"

# Commit an action
modal c commit --action '{"type":"DEPOSIT","amount":100}' --sign alice.passfile
```

### View History

```bash
# Show commit log
modal c log

# Show detailed log
modal c log --verbose
```

### Check Status

```bash
modal c status
modal status  # shortcut when in contract directory
```

### View Differences

```bash
# Show changes since last commit
modal c diff
```

### Push to Network/Hub

```bash
# Push to a hub
modal c push http://hub.example.com/contracts/my-contract

# Push to chain validators
modal c push /ip4/validator.modality.network/...
```

### Pull from Network/Hub

```bash
# Pull latest commits
modal c pull http://hub.example.com/contracts/my-contract
```

### Get Contract Info

```bash
# Get contract ID
modal c id

# Get current commit ID  
modal c commit-id

# Get state value
modal c get /parties/alice.id
```

### Pack/Unpack Contracts

```bash
# Pack contract into a portable file
modal c pack --output my-contract.contract

# Unpack a contract file
modal c unpack my-contract.contract --output ./restored-contract
```

---

## Identity Commands (`modal id`)

### Create a New Identity

```bash
# Create with default path
modal id create --path alice.passfile

# Create with password protection
modal id create --path alice.passfile --password
```

A passfile contains the ed25519 private key for signing.

### Derive Sub-Identity

```bash
modal id derive --path alice.passfile --sub "escrow-key" --output alice-escrow.passfile
```

Creates a deterministic sub-key from a parent key.

### Get Public ID

```bash
# Get the public ID (ed25519 public key)
modal id get --path alice.passfile
# Output: ed25519:abc123...
```

---

## Passfile Commands (`modal passfile`)

### Encrypt a Passfile

```bash
modal passfile encrypt --path alice.passfile --password
```

### Decrypt a Passfile

```bash
modal passfile decrypt --path alice.passfile.enc --password
```

---

## Predicate Commands (`modal predicate`)

### List Available Predicates

```bash
modal predicate list
```

### Get Predicate Info

```bash
modal predicate info signed_by
modal predicate info threshold
```

### Test a Predicate

```bash
modal predicate test signed_by --args '{"id_path":"/parties/alice.id"}' --commit ./test-commit
```

### Create Custom Predicate

```bash
modal predicate create --name my_predicate --output ./predicates/my_predicate
```

---

## Node Commands (`modal node`)

### Create a Node

```bash
modal node create --path ./my-node
```

### Start/Stop a Node

```bash
modal node start --path ./my-node
modal node stop --path ./my-node
modal node restart --path ./my-node
```

### View Node Info

```bash
modal node info --path ./my-node
modal node address --path ./my-node
modal node logs --path ./my-node
```

### Run a Node (Foreground)

```bash
# Run as miner
modal node run-miner --path ./my-node

# Run as validator (observes, doesn't mine)
modal node run-validator --path ./my-node

# Run as observer
modal node run-observer --path ./my-node
```

### Sync with Network

```bash
modal node sync --path ./my-node
```

---

## Network Commands (`modal net`)

### Network Info

```bash
modal net info
```

### Storage Stats

```bash
modal net storage
```

---

## Common Patterns

### Create and Initialize a Contract

```bash
mkdir my-contract && cd my-contract
modal c create
modal id create --path alice.passfile
modal id create --path bob.passfile
modal c checkout
modal c set /parties/alice.id $(modal id get --path ./alice.passfile)
modal c set /parties/bob.id $(modal id get --path ./bob.passfile)
modal c commit --all --sign alice.passfile -m "Initial setup"
```

### Multi-Party Signing Flow

```bash
# Alice creates and signs
modal c commit --all --sign alice.passfile -m "Alice's terms"

# Share contract with Bob (pack/send/unpack or push/pull via hub)
modal c pack --output contract-for-bob.contract

# Bob adds their rules and signs
modal c commit --all --sign bob.passfile -m "Bob agrees"
```

### Execute an Action

```bash
# Perform DEPOSIT action
modal c commit --action '{"type":"DEPOSIT"}' --sign alice.passfile -m "Deposited funds"
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MODAL_NODE_PATH` | Default node directory |
| `MODAL_NETWORK` | Default network (mainnet/testnet) |
| `MODAL_HUB_URL` | Default hub URL for push/pull |

---

## Next Steps

- **[Language Reference](../language/README.md)** — Model and rule syntax
- **[Tutorials](../tutorials/)** — Step-by-step guides
