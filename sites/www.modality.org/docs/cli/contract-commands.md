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
| `--signer <ID or PASSFILE>` | A key allowed to extend the contract; repeat for several. The genesis commit posts the keys at `/signers/<n>.id`, a model and the rule that every later commit is signed by one of them. The contract's key signs the set for the hash lane (`.contract/signer_set.json`). Fixed at creation |
| `--key <PASSFILE>` | The contract's key, instead of a new one: the contract's id is that key's id. A miner makes the contract its blocks nominate this way, to take their MOD. One contract per key |
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
| `--sign <PASSFILE>` | Sign with a passfile path or identity name; repeat to attach multiple signatures. Each signs the contract id and the whole commit except its signatures (see [Commit signatures](../reference/standard-predicates.md#commit-signatures)) |
| `--all`, `-a` | Commit all changed `state/`, `rules/`, `reposts/`, and `model/default.modality` files. Staged REPOST dests emit `method: repost`. With `--method create`, `send`, `recv` or `invoke`, that action joins the same commit, so a `SEND` and the flag it sets are one commit. |
| `--message`, `-m <MSG>` | Commit message |
| `--action <JSON>` | Commit an inline JSON domain action or read it from a `.json` file path |
| `--asset-id <ASSET_ID>` | Asset ID for `CREATE` and `SEND` commits; on a `RECV`, the asset it states it receives |
| `--asset-contract <CONTRACT_ID>` | Creator of a received asset: on a `SEND`, the held asset to send on; on a `RECV`, the creator it states. Omit it for the contract's own asset |
| `--memo <JSON>` | JSON the receiver of a `SEND` reads, recorded with it; on a `RECV`, the memo it states the `SEND` carries |
| `--quantity <QUANTITY>` | Asset quantity for `CREATE` commits |
| `--divisibility <DIVISIBILITY>` | Asset divisibility for `CREATE` commits |
| `--to-contract <TO_CONTRACT>` | Destination contract ID for `SEND` commits |
| `--amount <AMOUNT>` | Amount for `SEND` commits; on a `RECV`, the amount it states it receives. Apply refuses a `RECV` whose statement differs from its `SEND` |
| `--send-commit-id <SEND_COMMIT_ID>` | Source `SEND` commit ID for `RECV` commits |
| `--send-index <N>` | Which `SEND` of that commit a `RECV` takes, from 0 (default); emitted `SEND`s count in `invoke` order |
| `--theory <v0\|v2\|v3>` | Predicate theory local verify runs (default `v3`, what the testnet runs). Use `v0` for a network whose `network.json` leaves `predicate_theory_version` unset, such as the bundled devnets, and `v2` for a network that sets `v2`. `v3` is `v2` with numbers compared exactly. `v2` and `v3` refuse dead model edges that `v0` accepts, and accept rules that `v0` refuses, such as `always([+SEND -any_signed(/claimants)] false)`. See [Predicate theory](../reference/predicate-theory.md) |

**Examples:**
```bash
# Commit all changes with signature
modal c commit --all --sign alice -m "Add escrow rules"

# Commit all changes with multiple member signatures
modal c commit --all --sign alice --sign bob -m "Replace witness"

# Pay out and mark the flag in one commit
modal c commit --all --method send --asset-id drops --to-contract <WALLET_ID> --amount 10 --sign carol

# Send on 100 of a received asset, with a memo for the receiver
modal c commit --method send --asset-contract <CREATOR_ID> --asset-id tokA --to-contract <POOL_ID> --amount 100 --memo '{"op":"swap","min_out":300}'

# Receive it, stating what arrives; apply refuses the RECV if it differs
modal c commit --method recv --send-commit-id <SEND_ID> --asset-contract <CREATOR_ID> --asset-id tokB --amount 362

# Commit one state file
modal c commit --path /notes.text --value "signed update" --sign alice

# Commit a domain action
modal c commit --action '{"type":"DEPOSIT","amount":100}' --sign alice
```

Under the default `--theory v3`, `modal c commit` refuses a model edge whose
labels can never hold together, such as `+num_lt(/escrow/paid.num,"100")` beside
`+num_gte(/escrow/paid.num,"100")`. With `--theory v0`, the commit is accepted
and the output previews what `v3` would change about it, as a warning.
Sequencers and validators refuse the commit if the network sets
`predicate_theory_version` to `v3`, as the testnet does. With `--output json`, the findings are
under `theory_preview`.

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
modal status  # alias for modal c status
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

## Repost

```bash
modal c repost <SOURCE_CONTRACT> <SOURCE_PATH> [DEST_PATH]
```

Snapshot a value from another contract into this one so formulas can name the dest path.

**Arguments:**
| Arg | Description |
|-----|-------------|
| `SOURCE_CONTRACT` | Source contract ID |
| `SOURCE_PATH` | Path on the source (e.g. `/parties/alice.id`) |
| `DEST_PATH` | Optional dest in this contract. Default: `/reposts/<source_id><source_path>` |

**Options:**
| Option | Description |
|--------|-------------|
| `--from-dir <DIR>` | Read the source from a local contract directory instead of the hub |
| `--dir <DIR>` | Dest contract directory (defaults to current directory) |

```bash
modal c repost abc123 /notes/hello.text
modal c repost abc123 /parties/alice.id /parties/alice.id
modal c repost abc123 /notes/hello.text --from-dir ../source-contract
modal commit --all
```

## Add Rule

```bash
modal c add-rule --name <NAME> [OPTIONS] <FORMULA>
```

Write a named rule file under `rules/`. The command creates `rules/` if needed
and wraps the formula as `export default rule { starting_at $PARENT ... }`.

**Options:**
| Option | Description |
|--------|-------------|
| `--name <NAME>` | Rule name written as `rules/<name>.modality` |
| `--starting-at <ANCHOR>` | Rule anchor; only `$PARENT` (the default) is supported |
| `--dir <DIR>` | Contract directory (defaults to current directory) |

```bash
modal c add-rule --name authorized \
  'always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)'
```

Existing rule files are not overwritten. Run `modal c commit --all` after adding
a rule.

## AI

Configure a provider first with [`modal ai set`](/docs/cli/ai-commands). Then:

```bash
modal c ai suggest-rule <PROMPT>
```

Suggest a Modality rule formula from a plain-language prompt. Use the printed
formula with `modal c add-rule`. Encodings follow the
[formula cookbook](/docs/language/formula-cookbook); witness models follow the
[model cookbook](/docs/language/model-cookbook). If no provider is configured,
the command fails with a hint to run
`modal ai set --provider openai|anthropic|grok|bedrock|ollama|cursor-agent`.

```bash
modal c ai suggest-rule "after this commit either alice or bob must sign"
```

```
always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
```

That printed formula is example output; yours may differ.

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
modal c set-named-id /parties/alice.id alice
modal c set-named-id /parties/alice.id example/alice
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

Push commits to a hub or to chain sequencers.

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
| `--reveal` | Push the commits as reveals of hash commitments made with `anchor`. The node refuses a reveal whose body does not hash to its commit id, or whose hash was never anchored. Chain remotes only |

A contract's log is linear. On chain sequencers, a commit must extend the
contract's current head: if another commit was already sequenced on the same
parent, the sequencer refuses yours ("forks the contract"). Pull, then commit
again on the new head.

A commit's id is the SHA-256 of the commit as compact JSON,
`{"body":[...],"head":{...}}`: each action's fields and the head's in the order
`modal` writes them, and the keys of every object inside a value sorted. A
node recomputes the id from the body and head it receives and refuses a commit
that does not hash to it; nothing of that push is queued. Write commits with
`modal c commit`, or hash them the same way.

On chain remotes, `push`, `pull`, `anchor` and `replay` start a short-lived
node. It logs warnings and errors to stderr, so stdout carries only the
command's output (`--output json` parses as is). Set `RUST_LOG=info` for more.

## Anchor

```bash
modal c anchor [OPTIONS]
```

Anchor commit hashes on a network's hash lane, without their bodies. For each
commit, `anchor` signs a hash commitment, grinds its hashtax against the
node's current epoch anchor, and submits it. A certified sequencer block
orders the hash. Send the bodies later with `push --reveal`. See
[Hash Commitments](../concepts/hash-commitments.md).

**Options:**
| Option | Description |
|--------|-------------|
| `--remote <MULTIADDR>` | Target node (defaults to the remote's URL; not saved) |
| `--remote-name <NAME>` | Remote name (default: `origin`) |
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--node-dir <DIR>` | Node directory for identity/config |
| `--commit <ID>` | Commit to anchor; repeat for several. Defaults to the commits not yet pushed |
| `--sign <PASSFILE>` | Key that signs the hash commitments. Defaults to a one-off key. For a contract created with `--signer`, one of its signers |
| `--bits <N>` | Leading zero bits of work to grind for (default: the network's floor). More work wins a full block |
| `--status` | Show what the node has for each commit (`unknown`, `anchored`, `revealed`) instead of anchoring |
| `--output <FORMAT>` | Output format: `text` or `json` |

```bash
modal c anchor --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW...
modal c anchor --status --commit <commit id>
modal c push --reveal
```

A network without a hash lane refuses `anchor`. For a contract created with
`--signer`, the genesis hash commitment posts the signer set with the
contract key's signature. Once it is certified, a hash commitment signed by
any other key is refused.

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
| `--contract-id <ID>` | Start a copy of this contract in `--dir`, which must not hold one yet, and pull its sequenced commits from `--remote` (saved as the remote) |
| `--output <FORMAT>` | Output format: `text` or `json` |

A node returns a contract's sequenced commits in log order, each after its
parent. Pull checks that each hashes to its id, moves `HEAD` when it was
unset or at the last pulled commit, and writes the state and rules files.

```bash
# Anyone with the contract id can take a copy and commit on its head
modal c pull --contract-id 12D3KooW... --dir ./pool --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW...
```

## Replay

```bash
modal c replay [OPTIONS]
```

Fetch a sequenced contract prefix (and posted WASM, if any) and re-check it
locally. The original node is not trusted: the same accumulated rules used on
sequenced apply are run again. That is independent replay.

```bash
# Stranger: fetch from a sequencer and verify
modal c replay --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/<peer> \
  --contract-id <id> --through <commit> --save prefix.json

# Later, offline
modal c replay --artifact prefix.json
```

**Options:**
| Option | Description |
|--------|-------------|
| `--artifact <FILE>` | Verify a saved replay artifact (no network) |
| `--save <FILE>` | Write the fetched or local artifact as JSON |
| `--remote <MULTIADDR>` | Sequencer to fetch `/contract/replay` from |
| `--contract-id <ID>` | Contract id when fetching without `--dir` |
| `--through <COMMIT>` | Sequenced tip to replay through |
| `--dir <DIR>` | Local contract directory (used when not fetching) |
| `--node-dir <DIR>` | Node directory for identity/config on P2P remotes |
| `--output <FORMAT>` | Output format: `text` or `json` |

## Theory

```bash
modal c theory [OPTIONS]
```

Show what the predicate theory derives from the accepted contract:
- dead edges: model transitions whose labels no commit can satisfy together,
  including any transition that needs a predicate the validator never
  evaluates (see [Standard Predicates](../reference/standard-predicates.md#implementation-status));
- edges dead after a step: transitions whose labels contradict what every way
  into their state keeps unchanged (through `-modifies`), so no run takes them
  (a warning; contracts may end);
- committed `.theory.json` declarations the theory cannot read. The validator
  does not evaluate `wasm` yet, so a `+wasm(...)` transition is dead whatever
  its declaration says;
- each move out of the current state, marked as `open`, `blocked`, or `forced`.

A move is `blocked` when accepted state already makes one of its labels false.
It is `forced` when it is the only open move out of a state that has more than
one. The view is read-only. Sequencers and validators enforce the network's
`predicate_theory_version` (V0 unless the network names one).

**Options:**
| Option | Description |
|--------|-------------|
| `--dir <DIR>` | Contract directory (defaults to current directory) |
| `--theory <VERSION>` | Theory version to preview: `v3` (default), `v2`, `v1`, or `v0`. Networks refuse `v1`. v3's entailment is v2's |
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
