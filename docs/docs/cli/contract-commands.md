---
sidebar_position: 2
title: Contract Commands
---

# Contract Commands (`modal contract` / `modal c`)

Manage contracts — create, commit, push, pull, and inspect.

## Create

```bash
modal c create [OPTIONS]
```

Creates a new contract in the current directory.

**What it creates:**
```
.contract/           # Contract metadata
├── config.json      # Contract configuration
├── commits/         # Commit storage
└── HEAD             # Current commit reference
state/               # Working state directory
```

**Options:**
| Option | Description |
|--------|-------------|
| `--name <NAME>` | Contract name |
| `--template <TEMPLATE>` | Initialize from template |

## Commit

```bash
modal c commit [OPTIONS]
```

Create a new commit from staged changes.

**Options:**
| Option | Description |
|--------|-------------|
| `--all`, `-a` | Commit all changes (state + rules + model) |
| `--state` | Commit only state changes |
| `--rules` | Commit only rule changes |
| `--model` | Commit only model changes |
| `--sign <PASSFILE>` | Sign commit with passfile |
| `--message`, `-m <MSG>` | Commit message |
| `--action <JSON>` | Commit a domain action |

**Examples:**
```bash
# Commit all changes with signature
modal c commit --all --sign alice.passfile -m "Add escrow rules"

# Commit a domain action
modal c commit --action '{"type":"DEPOSIT","amount":100}' --sign alice.passfile

# Commit only state changes
modal c commit --state -m "Update configuration"
```

## Checkout

```bash
modal c checkout [OPTIONS]
```

Extract committed state to the working `state/` directory.

**Options:**
| Option | Description |
|--------|-------------|
| `--commit <HASH>` | Checkout specific commit |
| `--force` | Overwrite local changes |

## Status

```bash
modal c status
modal status  # shortcut when in contract directory
```

Shows:
- Current commit
- Modified files
- Staged changes
- Rule validation status

## Diff

```bash
modal c diff [OPTIONS]
```

Show changes between working state and committed state.

**Options:**
| Option | Description |
|--------|-------------|
| `--commit <HASH>` | Compare against specific commit |
| `--stat` | Show only file statistics |

## Log

```bash
modal c log [OPTIONS]
```

Show commit history.

**Options:**
| Option | Description |
|--------|-------------|
| `--verbose`, `-v` | Show full commit details |
| `--limit <N>` | Limit number of commits shown |
| `--oneline` | Compact format |

**Example output:**
```
abc123 (HEAD) Add escrow rules [alice] 2024-01-15 10:30:00
def456 Initial contract setup [alice] 2024-01-15 10:00:00
```

## Set

```bash
modal c set <PATH> [VALUE] [OPTIONS]
```

Set a state file value.

**Options:**
| Option | Description |
|--------|-------------|
| `--file <FILE>` | Read value from file |
| `--type <TYPE>` | Explicit path type |

**Examples:**
```bash
# Set text value
modal c set /config/name.text "My Contract"

# Set from file
modal c set /data/config.json --file ./local-config.json

# Set boolean
modal c set /flags/active.bool true
```

## Set Named ID

```bash
modal c set-named-id <PATH> --named <NAME>
```

Set a `.id` file from a named passfile in your passfile directory.

```bash
modal c set-named-id /parties/alice.id --named alice
# Uses ~/.modal/passfiles/alice.passfile
```

## Get

```bash
modal c get <PATH> [OPTIONS]
```

Get contract or state information.

**Options:**
| Option | Description |
|--------|-------------|
| `--commit <HASH>` | Get from specific commit |
| `--raw` | Output raw bytes |

```bash
modal c get /parties/alice.id
modal c get /data/config.json --commit abc123
```

## ID Commands

```bash
# Get contract ID
modal c id

# Get current commit ID
modal c commit-id
```

## Push

```bash
modal c push <REMOTE> [OPTIONS]
```

Push commits to a hub or chain validators.

**Remote formats:**
- Hub: `http://hub.example.com/contracts/<id>`
- Chain: `/ip4/<addr>/tcp/<port>/p2p/<peer_id>`

**Options:**
| Option | Description |
|--------|-------------|
| `--sign <PASSFILE>` | Sign push request |
| `--force` | Force push |

## Pull

```bash
modal c pull <REMOTE> [OPTIONS]
```

Pull commits from a hub or chain.

**Options:**
| Option | Description |
|--------|-------------|
| `--checkout` | Checkout after pulling |

## Pack / Unpack

```bash
# Pack contract into portable file
modal c pack --output contract.modal

# Unpack contract file
modal c unpack contract.modal --output ./my-contract
```

## Assets

```bash
modal c assets [OPTIONS]
```

Manage contract assets.

**Options:**
| Option | Description |
|--------|-------------|
| `--list` | List all assets |
| `--add <PATH>` | Add asset |
| `--remove <PATH>` | Remove asset |

## WASM Upload

```bash
modal c wasm-upload <WASM_FILE> [OPTIONS]
```

Upload a WASM module for custom predicates.

**Options:**
| Option | Description |
|--------|-------------|
| `--name <NAME>` | Module name |
| `--sign <PASSFILE>` | Sign upload |
