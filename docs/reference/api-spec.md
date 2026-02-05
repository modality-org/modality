# Modality API Specification

**Version:** 0.1.0  
**Base URL:** `https://api.modality.org/v1`

## Overview

Stateless HTTP API for creating and interacting with Modality contracts. No installation required - just HTTP requests with ed25519 signatures.

## Authentication

All mutating requests must include a signature header:

```
X-Modality-Pubkey: <hex-encoded-ed25519-pubkey>
X-Modality-Signature: <hex-encoded-signature-of-request-body>
```

Read-only endpoints (GET) don't require authentication.

---

## Endpoints

### Contracts

#### Create Contract

```
POST /contracts
```

Create a new contract from a template or custom model.

**Request:**
```json
{
  "template": "escrow",
  "params": {
    "buyer": "abc123...",
    "seller": "def456...",
    "arbiter": "789ghi..."
  }
}
```

Or with custom model:
```json
{
  "model": "model MyContract { init --> ready: +START ... }",
  "rules": [
    "rule protect { formula { always (...) } }"
  ]
}
```

**Response:** `201 Created`
```json
{
  "contract_id": "c_8f3a2b1c",
  "model": "model Escrow { ... }",
  "rules": ["..."],
  "state": {
    "current": "init",
    "paths": {}
  },
  "created_at": "2026-02-05T06:30:00Z"
}
```

---

#### Get Contract

```
GET /contracts/{contract_id}
```

**Response:** `200 OK`
```json
{
  "contract_id": "c_8f3a2b1c",
  "model": "model Escrow { ... }",
  "rules": ["..."],
  "state": {
    "current": "deposited",
    "paths": {
      "/escrow/amount.balance": 100,
      "/escrow/buyer.pubkey": "abc123..."
    }
  },
  "commit_count": 3,
  "created_at": "2026-02-05T06:30:00Z",
  "updated_at": "2026-02-05T06:35:00Z"
}
```

---

#### Get Contract State

```
GET /contracts/{contract_id}/state
```

Returns current state and valid next actions.

**Response:** `200 OK`
```json
{
  "current_state": "deposited",
  "paths": {
    "/escrow/amount.balance": 100,
    "/escrow/buyer.pubkey": "abc123..."
  },
  "valid_actions": [
    {
      "action": "DELIVER",
      "required_signer": "seller",
      "next_state": "delivered"
    },
    {
      "action": "CANCEL",
      "required_signer": "buyer",
      "next_state": "cancelled"
    }
  ]
}
```

---

#### Get Contract Log

```
GET /contracts/{contract_id}/log
```

Returns commit history.

**Query params:**
- `limit` (int, default 50)
- `offset` (int, default 0)

**Response:** `200 OK`
```json
{
  "commits": [
    {
      "index": 0,
      "hash": "a1b2c3...",
      "method": "genesis",
      "timestamp": "2026-02-05T06:30:00Z"
    },
    {
      "index": 1,
      "hash": "d4e5f6...",
      "method": "post",
      "path": "/escrow/amount.balance",
      "value": 100,
      "signer": "abc123...",
      "timestamp": "2026-02-05T06:31:00Z"
    }
  ],
  "total": 2
}
```

---

### Commits

#### Create Commit

```
POST /contracts/{contract_id}/commits
```

Submit a signed commit to the contract.

**Request:**
```json
{
  "method": "post",
  "path": "/delivered.bool",
  "value": true,
  "action_labels": ["DELIVER"]
}
```

**Headers:**
```
X-Modality-Pubkey: def456...
X-Modality-Signature: <signature-of-request-body>
```

**Response:** `201 Created`
```json
{
  "commit_hash": "g7h8i9...",
  "index": 4,
  "new_state": "delivered",
  "timestamp": "2026-02-05T06:40:00Z"
}
```

**Error Response:** `400 Bad Request`
```json
{
  "error": "invalid_transition",
  "message": "Action DELIVER not valid from state 'init'",
  "valid_actions": ["DEPOSIT"]
}
```

---

### Templates

#### List Templates

```
GET /templates
```

**Response:** `200 OK`
```json
{
  "templates": [
    {
      "id": "escrow",
      "name": "Escrow",
      "description": "Two-party escrow with optional arbiter",
      "params": ["buyer", "seller", "arbiter?"]
    },
    {
      "id": "milestone",
      "name": "Milestone Payment",
      "description": "Multi-stage payment on deliverables",
      "params": ["client", "contractor", "milestones"]
    }
  ]
}
```

#### Get Template

```
GET /templates/{template_id}
```

**Response:** `200 OK`
```json
{
  "id": "escrow",
  "name": "Escrow",
  "description": "Two-party escrow with optional arbiter",
  "params": [
    {"name": "buyer", "type": "pubkey", "required": true},
    {"name": "seller", "type": "pubkey", "required": true},
    {"name": "arbiter", "type": "pubkey", "required": false}
  ],
  "model": "model Escrow { ... }",
  "rules": ["..."]
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": "error_code",
  "message": "Human readable description",
  "details": {}
}
```

| Status | Error Code | Description |
|--------|------------|-------------|
| 400 | `invalid_request` | Malformed request body |
| 400 | `invalid_signature` | Signature doesn't match pubkey |
| 400 | `invalid_transition` | Action not valid from current state |
| 400 | `invalid_model` | Model syntax error |
| 404 | `not_found` | Contract or template not found |
| 409 | `conflict` | Concurrent modification |
| 429 | `rate_limited` | Too many requests |

---

## Rate Limits

| Tier | Requests/min | Contracts |
|------|--------------|-----------|
| Free | 60 | 10 |
| Pro | 600 | unlimited |

Rate limit headers included in all responses:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1706853600
```

---

## Example: Full Escrow Flow

```bash
# 1. Create escrow contract
curl -X POST https://api.modality.org/v1/contracts \
  -H "Content-Type: application/json" \
  -d '{"template": "escrow", "params": {"buyer": "aaa...", "seller": "bbb..."}}'

# Response: {"contract_id": "c_123", ...}

# 2. Buyer deposits (signed request)
curl -X POST https://api.modality.org/v1/contracts/c_123/commits \
  -H "X-Modality-Pubkey: aaa..." \
  -H "X-Modality-Signature: <sig>" \
  -d '{"method": "post", "path": "/deposit.balance", "value": 100, "action_labels": ["DEPOSIT"]}'

# 3. Check state
curl https://api.modality.org/v1/contracts/c_123/state

# Response: {"current_state": "deposited", "valid_actions": ["DELIVER", "CANCEL"]}

# 4. Seller delivers (signed request)
curl -X POST https://api.modality.org/v1/contracts/c_123/commits \
  -H "X-Modality-Pubkey: bbb..." \
  -H "X-Modality-Signature: <sig>" \
  -d '{"method": "post", "path": "/delivered.bool", "value": true, "action_labels": ["DELIVER"]}'

# 5. Buyer releases (signed request)  
curl -X POST https://api.modality.org/v1/contracts/c_123/commits \
  -H "X-Modality-Pubkey: aaa..." \
  -H "X-Modality-Signature: <sig>" \
  -d '{"action_labels": ["RELEASE"]}'

# Contract complete!
```

---

## Future Endpoints (v2)

- `POST /contracts/synthesize` - NL → contract
- `GET /contracts/search` - discover contracts
- `POST /contracts/{id}/subscribe` - webhooks
