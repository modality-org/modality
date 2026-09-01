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
| `--dir <DIR>` | Directory path where the contract will be created (defaults to current directory) |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Commit

```bash
modal c commit [OPTIONS]
```

Create a new commit from the contract working directories, a single state path,
or an inline domain action.

**Options:**
| Option | Description |
|--------|-------------|
| `--path <PATH>` | State path to write for a single `POST`-style commit |
| `--value <VALUE>` | Value for the single-path commit; strings, numbers, and JSON are accepted |
| `--method <METHOD>` | Commit method for the single-path commit (default: `post`) |
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--output <FORMAT>` | Output format: `text` or `json` |
| `--sign <PASSFILE>` | Sign commit with a passfile; repeat to attach multiple signatures |
| `--all`, `-a` | Commit all changed `state/`, `rules/`, and `model/default.modality` files |
| `--message`, `-m <MSG>` | Commit message |
| `--action <JSON>` | Commit an inline JSON domain action or read it from a `.json` file path |
| `--asset-id <ASSET_ID>` | Asset ID for `CREATE` commits |
| `--quantity <QUANTITY>` | Asset quantity for `CREATE` commits |
| `--divisibility <DIVISIBILITY>` | Asset divisibility for `CREATE` commits |
| `--to-contract <TO_CONTRACT>` | Destination contract ID for `SEND` commits |
| `--amount <AMOUNT>` | Amount for `SEND` commits |
| `--send-commit-id <SEND_COMMIT_ID>` | Source `SEND` commit ID for `RECV` commits |

**Examples:**
```bash
# Commit all changes with signature
modal c commit --all --sign alice.passfile -m "Add escrow rules"

# Commit all changes with multiple member signatures
modal c commit --all --sign alice.passfile --sign bob.passfile -m "Replace witness"

# Commit one state file
modal c commit --path /notes.text --value "signed update" --sign alice.passfile

# Commit a domain action
modal c commit --action '{"type":"DEPOSIT","amount":100}' --sign alice.passfile
```

## Checkout

```bash
modal c checkout [OPTIONS]
```

Extract committed state to the working `state/` directory.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |

## Status

```bash
modal c status [OPTIONS]
modal status  # shortcut when in contract directory
```

Shows:
- Current commit
- Modified files
- Staged changes
- Rule validation status

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--remote <NAME>` | Remote name to compare with (default: `origin`) |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Diff

```bash
modal c diff [OPTIONS]
```

Show changes between working state and committed state.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Log

```bash
modal c log [OPTIONS]
```

Show commit history.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--limit <N>`, `-n <N>` | Limit number of commits shown |
| `--output <FORMAT>` | Output format: `text` or `json` |

**Example output:**
```
abc123 (HEAD) Add escrow rules [alice] 2024-01-15 10:30:00
def456 Initial contract setup [alice] 2024-01-15 10:00:00
```

## Set

```bash
modal c set [OPTIONS] <PATH> <VALUE>
```

Set a state file value.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |

**Examples:**
```bash
# Set text value
modal c set /config/name.text "My Contract"

# Set boolean
modal c set /flags/active.bool true
```

## Set Named ID

```bash
modal c set-named-id [OPTIONS] <PATH> <NAME>
```

Set a `.id` file from a passfile path or a passfile name that resolves in the
standard passfile locations.

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |

```bash
modal c set-named-id /parties/alice.id alice.passfile
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
modal c push [OPTIONS]
```

Push commits to a hub or chain validators.

**Remote formats:**
- Hub: `http://hub.example.com/contracts/<id>`
- Chain: `/ip4/<addr>/tcp/<port>/p2p/<peer_id>`

**Options:**
| Option | Description |
|--------|-------------|
| `--remote <URL>` | Target node multiaddress or hub URL; also saves it under the remote name |
| `--remote-name <NAME>` | Remote name (default: `origin`) |
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--node-dir <DIR>` | Node directory for identity/config when using P2P remotes |
| `--hub-creds <FILE>` | Hub credentials file for HTTP hub remotes |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Pull

```bash
modal c pull [URL] [OPTIONS]
```

Pull commits from a hub or chain.

**Options:**
| Option | Description |
|--------|-------------|
| `[URL]` | Full contract URL to clone, such as `https://hub/contracts/<id>` |
| `--remote <URL>` | Target node multiaddress or hub URL |
| `--remote-name <NAME>` | Remote name (default: `origin`) |
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--node-dir <DIR>` | Node directory for identity/config when using P2P remotes |
| `--hub-creds <FILE>` | Hub credentials file for HTTP hub remotes |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Pack / Unpack

```bash
# Pack contract into portable file
modal c pack --output contract.modal

# Unpack contract file
modal c unpack contract.modal --output ./my-contract
```

**Options:**
| Command | Option | Description |
|---------|--------|-------------|
| `pack` | `--output <FILE>`, `-o <FILE>` | Output `.contract` file path |
| `pack` | `--dir <DIR>` | Contract directory (defaults to current directory) |
| `unpack` | `<INPUT>` | Input `.contract` file path |
| `unpack` | `--output <DIR>`, `-o <DIR>` | Output directory |
| `unpack` | `--force` | Overwrite an existing output directory |

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
