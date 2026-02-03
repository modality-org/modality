# Commit Methods

Modality contracts are append-only logs of commits. Each commit contains one or more **actions**, and each action has a **method** that determines what it does.

## POST

The most common method - writes data to a path in the contract state.

```json
{
  "method": "post",
  "path": "/users/alice.id",
  "value": "12D3KooWAbCdEfGhIjKlMnOpQrStUvWxYz..."
}
```

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

## RULE

Adds a temporal logic constraint to the contract. Rules are accumulated over time and all must be satisfied.

```json
{
  "method": "rule",
  "path": "/rules/auth.modality",
  "value": "always ([<+signed_by(/users/alice.id)>] true)"
}
```

Rules must have paths ending in `.modality`.

## REPOST

Copies data from another contract into a local namespace. This enables cross-contract data sharing while maintaining clear provenance.

```json
{
  "method": "repost",
  "path": "$abc123def:/announcements/latest.text",
  "value": "Hello from the other contract!"
}
```

### Path Format

REPOST paths use a special namespace format:

```
$<source_contract_id>:/<remote_path>
```

- `$` - Indicates this is reposted (external) data
- `<source_contract_id>` - The contract ID where the data originated
- `:` - Separator
- `/<remote_path>` - The original path in the source contract

### Local Storage

Reposted data is stored locally in the `reposts/` directory:

```
reposts/
  <contract_id>/
    <path>/
      file.ext
```

### CLI Usage

```bash
# Repost data from another contract
modal contract repost \
  --from-contract abc123def456 \
  --from-path /announcements/latest.text \
  --value "The announcement content"

# With custom destination path
modal contract repost \
  --from-contract abc123def456 \
  --from-path /data/config.json \
  --to-path '$abc123def456:/imported/config.json' \
  --value '{"setting": "value"}'

# Signed repost
modal contract repost \
  --from-contract abc123def456 \
  --from-path /messages/hello.text \
  --value "Hello!" \
  --sign alice.passfile
```

### Use Cases

1. **Cross-contract references**: Include data from one contract in another's state
2. **Data mirroring**: Keep a local copy of important external data
3. **Audit trails**: Track the provenance of imported data
4. **Agent coordination**: Share information between contracts managed by different agents

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

Receives assets from a SEND in another contract.

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
