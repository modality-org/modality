# Commit Methods

Modality contracts are append-only logs of commits. Each commit contains one or more **actions**, and each action has a **method** that determines what it does.

## POST

The most common method — writes data to a path in **this** contract's state.

```json
{
  "method": "post",
  "path": "/users/alice.id",
  "value": "12D3KooWAbCdEfGhIjKlMnOpQrStUvWxYz..."
}
```

After the commit is accepted, formulas on this contract can name that path
(`signed_by(/users/alice.id)`, `text_eq`, `state_exists`, …).

### Path Types

Paths must end with a known extension:

| Extension | Type | Example Value |
|-----------|------|---------------|
| `.bool` | Boolean | `true` or `false` |
| `.text` | Text string | `"Hello world"` |
| `.date` | Date | `"2024-01-15"` |
| `.datetime` | Date and time | `"2024-01-15T10:30:00Z"` |
| `.json` | JSON object | `{"key": "value"}` |
| `.md` | Markdown | `"# Title\n\nContent..."` |
| `.id` | Modality ID | `"12D3KooW..."` |
| `.wasm` | WebAssembly | Base64-encoded WASM |
| `.modality` | Rules/formulas | Modality syntax |

## MODEL

Sets or replaces the contract's state machine model. Unlike rules, models can be replaced.

```json
{
  "method": "model",
  "value": "model escrow { initial idle; idle -> funded [DEPOSIT]; funded -> released [RELEASE] }"
}
```

### CLI Usage

```bash
modal contract commit --method model \
  --value 'model members_only { initial active; active -> active [] }' \
  --sign alice
```

> **Note**: Models define structure but don't enforce security on their own. Use RULE methods to add protection that persists even if the model is replaced.

## RULE

Adds a temporal logic constraint to the contract. Rules are accumulated over time and all must be satisfied.

```json
{
  "method": "rule",
  "path": "/rules/auth.modality",
  "value": "always([<+signed_by(/users/alice.id)>] true)"
}
```

Rules must have paths ending in `.modality`.

## REPOST

Copies a **snapshot** of another contract's value into this contract so later
rules can name it like any other path. It is not a live pointer: later updates
on the source do not change this contract until you REPOST again.

```json
{
  "method": "repost",
  "path": "/reposts/abc123def456/announcements/latest.text",
  "value": "Hello from the other contract!",
  "source_contract": "abc123def456",
  "source_path": "/announcements/latest.text",
  "source_commit": "a1b2c3..."
}
```

- `path` — dest path in **this** contract (a normal `/...` path). Default dest
  is `/reposts/<source_id><source_path>`. Choose a path like
  `/parties/alice.id` when the imported value should *be* that identity.
- `value` — bytes copied from the source.
- `source_contract`, `source_path`, `source_commit` — provenance. The hub
  checks that the source contract at `source_commit` has `value` at
  `source_path`.

After accept, dest is ordinary accepted state. Example: REPOST Alice's key to
`/parties/alice.id`, then `signed_by(/parties/alice.id)` works. Method label
is `+REPOST` (not `+POST`). `modifies(dest)` matches. `post_to_path` stays
POST-only. Models that only allow `+POST` must also allow `+REPOST` (or an
unlabeled write loop) before import commits will pass.

### Working tree

Default dest files live under `reposts/<source_id>/...`. Custom dests live
under `state/` like POST. `modal commit --all` emits `method: "repost"` for
paths staged by `modal repost`.

### CLI Usage

```bash
# Default dest /reposts/<source_id>/announcements/latest.text
modal repost abc123def456 /announcements/latest.text

# Custom dest so formulas can say signed_by(/parties/alice.id)
modal repost abc123def456 /parties/alice.id /parties/alice.id

# Source is a local contract directory (no hub)
modal repost abc123def456 /notes/hello.text --from-dir ../source-contract

modal commit --all
```

## CREATE

Creates a new asset in the contract.

```json
{
  "method": "create",
  "value": {
    "asset_id": "token1",
    "quantity": 21000000,
    "divisibility": 100000000
  }
}
```

## SEND

Sends assets to another contract.

```json
{
  "method": "send",
  "value": {
    "asset_id": "token1",
    "to_contract": "target_contract_id",
    "amount": 1000
  }
}
```

## RECV

Receives assets from a SEND in another contract. The SEND commit must
already be sequenced. When the network requires validator certs, dest
apply also needs a prefix-cert supermajority through that SEND commit.

```json
{
  "method": "recv",
  "value": {
    "send_commit_id": "abc123..."
  }
}
```

## INVOKE

Executes a WASM program stored in the contract.

```json
{
  "method": "invoke",
  "path": "/__programs__/calculator.wasm",
  "value": {
    "args": {
      "operation": "add",
      "a": 5,
      "b": 3
    }
  }
}
```
