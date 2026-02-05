# Hub REST API Reference

The Modal Hub exposes a REST API for contract management and collaboration.

## Base URL

```
http://localhost:3000
```

(Default port when running `modal hub start`)

## Endpoints

### Health

#### GET /health

Check hub status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

### Contracts

#### POST /contracts

Create a new contract.

**Request:**
```json
{
  "model": "model Escrow { init --> deposited: +DEPOSIT ... }",
  "rules": ["export default rule { ... }"],
  "owner": "ed25519:abc123..."
}
```

**Response:**
```json
{
  "contract_id": "c_abc123",
  "created_at": 1707123456
}
```

#### GET /contracts/:id

Get contract details.

**Response:**
```json
{
  "contract_id": "c_abc123",
  "model": "model Escrow { ... }",
  "rules": ["..."],
  "state": {
    "current_state": "init",
    "valid_actions": [
      { "action": "DEPOSIT", "required_signer": "/users/buyer.id" }
    ]
  },
  "created_at": 1707123456
}
```

#### GET /contracts/:id/state

Get current contract state.

**Response:**
```json
{
  "current_state": "deposited",
  "valid_actions": [
    { "action": "DELIVER", "required_signer": "/users/seller.id" },
    { "action": "CANCEL", "required_signer": "/users/buyer.id" }
  ],
  "commit_count": 3
}
```

#### GET /contracts/:id/log

Get commit history.

**Query params:**
- `limit` (optional): Max commits to return
- `offset` (optional): Skip N commits

**Response:**
```json
{
  "commits": [
    {
      "index": 0,
      "hash": "abc123...",
      "method": "genesis",
      "timestamp": 1707123456
    },
    {
      "index": 1,
      "hash": "def456...",
      "method": "post",
      "path": "/state/deposited",
      "value": true,
      "signer": "ed25519:xyz...",
      "timestamp": 1707123500
    }
  ],
  "total": 2
}
```

#### POST /contracts/:id/commits

Submit a new commit.

**Request:**
```json
{
  "contract_id": "c_abc123",
  "method": "post",
  "path": "/state/current",
  "value": "deposited",
  "action_labels": ["DEPOSIT"],
  "signatures": {
    "/users/buyer.id": "sig_abc123..."
  }
}
```

**Response:**
```json
{
  "commit_hash": "ghi789...",
  "index": 2,
  "new_state": {
    "current_state": "deposited",
    "valid_actions": [...]
  },
  "timestamp": 1707123600
}
```

#### GET /contracts/:id/commits/:hash

Get a specific commit.

**Response:**
```json
{
  "index": 1,
  "hash": "def456...",
  "method": "post",
  "path": "/state/deposited",
  "value": true,
  "signer": "ed25519:xyz...",
  "timestamp": 1707123500
}
```

---

### Synthesis (NL → Contract)

#### POST /contracts/synthesize

Generate a Modality contract from natural language description.

**Request:**
```json
{
  "description": "Alice wants to buy a rare item from Bob. Alice should deposit funds first, then Bob delivers the item, and finally Alice releases the funds. If there's a dispute, Carol acts as arbiter.",
  "pattern_hint": "escrow"  // optional
}
```

**Response:**
```json
{
  "model": "model Escrow {\n  init --> deposited: +DEPOSIT +signed_by(\"/users/alice.id\")\n  deposited --> delivered: +DELIVER +signed_by(\"/users/bob.id\")\n  delivered --> complete: +RELEASE +signed_by(\"/users/alice.id\")\n  deposited --> disputed: +DISPUTE +signed_by(\"/users/alice.id\")\n  disputed --> complete: +RESOLVE +signed_by(\"/users/carol.id\")\n}",
  "rules": [
    "export default rule {\n  starting_at $PARENT\n  formula {\n    always ([<+signed_by(/users/alice.id)>] true implies eventually(<+RELEASE> true))\n  }\n}"
  ],
  "parties": ["alice", "bob", "carol"],
  "protections": {
    "alice": "Funds protected until delivery confirmed",
    "bob": "Payment guaranteed upon delivery",
    "carol": "Arbiter authority for disputes only"
  },
  "prompt": "..."  // The LLM prompt used (for debugging)
}
```

**Notes:**
- Requires `ANTHROPIC_API_KEY` environment variable
- Uses Claude to parse natural language and generate Modality syntax
- The `pattern_hint` field helps guide synthesis toward known patterns (escrow, swap, multisig, etc.)

---

### Templates

#### GET /templates

List available contract templates.

**Response:**
```json
[
  {
    "id": "escrow",
    "name": "Escrow",
    "description": "Two-party escrow with optional arbiter",
    "params": [
      { "name": "buyer", "type": "pubkey", "required": true },
      { "name": "seller", "type": "pubkey", "required": true },
      { "name": "arbiter", "type": "pubkey", "required": false }
    ]
  },
  {
    "id": "milestone",
    "name": "Milestone Payment",
    "description": "Multi-stage payment on deliverables",
    "params": [...]
  }
]
```

#### GET /templates/:id

Get template details including model and rules.

**Response:**
```json
{
  "id": "escrow",
  "name": "Escrow",
  "description": "Two-party escrow with optional arbiter",
  "params": [...],
  "model": "model Escrow { ... }",
  "rules": []
}
```

---

## Error Responses

All endpoints return errors in this format:

```json
{
  "error": "Contract not found",
  "code": "NOT_FOUND"
}
```

**Common error codes:**
- `NOT_FOUND` - Resource doesn't exist
- `INVALID_REQUEST` - Malformed request body
- `VALIDATION_FAILED` - Contract validation error (invalid action, missing signature, etc.)
- `UNAUTHORIZED` - Missing or invalid authentication

---

## Authentication

The Hub uses ed25519 key-based authentication with two tiers:

1. **Identity Key** - Long-term key for account identity
2. **Access Key** - Session key for API requests

See [Hub Authentication](./hub-authentication.md) for details.
