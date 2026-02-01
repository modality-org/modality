# Contract Hub

Centralized HTTP service for push/pull of Modality contracts with two-tier ed25519 authentication.

## Two-Tier Key Architecture

**Identity Key** (long-term)
- Proves ownership of contracts
- Used to create/revoke access keys
- Keep secure, use sparingly

**Access Key** (session)
- Used for day-to-day API authentication
- Can be rotated, expired, revoked
- Sign all API requests

## Quick Start

```bash
# Install dependencies
npm install

# Start server
npm start

# Or with custom port/data dir
PORT=3100 DATA_DIR=./data npm start
```

## Workflow

### 1. Generate Keys

```javascript
import { ContractHubClient } from './src/client.js';

// Generate identity keypair (keep private key VERY secure)
const identity = await ContractHubClient.generateKeypair();
console.log('Identity private key:', identity.privateKey); // SAVE SECURELY
console.log('Identity public key:', identity.publicKey);

// Generate access keypair (for API use)
const access = await ContractHubClient.generateKeypair();
console.log('Access public key:', access.publicKey);
```

### 2. Register Identity

```bash
curl -X POST http://localhost:3100/identity/register \
  -H "Content-Type: application/json" \
  -d '{"public_key": "IDENTITY_PUBLIC_KEY_HEX"}'
```

Returns: `{ "identity_id": "id_xxx", "public_key": "..." }`

### 3. Create Access Key

Sign with identity key to create an access key:

```bash
# Signature = identity_private_key.sign("create_access:" + access_public_key + ":" + timestamp)
curl -X POST http://localhost:3100/access/create \
  -H "Content-Type: application/json" \
  -d '{
    "identity_id": "id_xxx",
    "access_public_key": "ACCESS_PUBLIC_KEY_HEX",
    "timestamp": "1769957000000",
    "signature": "IDENTITY_SIGNATURE_HEX",
    "name": "laptop"
  }'
```

Returns: `{ "access_id": "acc_xxx", "identity_id": "id_xxx", "public_key": "..." }`

### 4. Use Access Key for API Calls

All authenticated endpoints require:
```
X-Access-Id: acc_xxx
X-Timestamp: <unix_ms>
X-Signature: access_private_key.sign(METHOD:PATH:TIMESTAMP:BODY_HASH)
```

## API Endpoints

### No Auth Required

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/identity/register` | Register identity key |
| POST | `/access/create` | Create access key (requires identity signature) |

### Auth Required (Access Key)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/access/list` | List your access keys |
| POST | `/access/revoke` | Revoke an access key |
| POST | `/contracts` | Create contract |
| GET | `/contracts` | List your contracts |
| GET | `/contracts/:id` | Get contract info |
| POST | `/contracts/:id/push` | Push commits |
| GET | `/contracts/:id/pull` | Pull commits |
| POST | `/contracts/:id/access` | Grant access to identity |

## Client Example

```javascript
import { ContractHubClient } from './src/client.js';

// Generate keypairs
const identityKeys = await ContractHubClient.generateKeypair();
const accessKeys = await ContractHubClient.generateKeypair();

const client = new ContractHubClient('http://localhost:3100');

// Register identity
const { identity_id } = await client.registerIdentity(identityKeys.publicKey);

// Create access key (signed by identity)
const { access_id } = await client.createAccessKey(
  identity_id, 
  accessKeys.publicKey,
  identityKeys.privateKey  // Signs the creation request
);

// Configure client with access credentials
client.accessId = access_id;
client.privateKey = accessKeys.privateKey;

// Now use the API
const { contract_id } = await client.createContract('My Contract');
await client.push(contract_id, commits);
await client.pull(contract_id);

// Grant access to another identity
await client.grantAccess(contract_id, 'id_other_user', 'read');
```

## Commit Validation

The hub validates all commits on push by default:

| Check | Description |
|-------|-------------|
| **Parent chain** | First commit must have parent = current head (or null). Subsequent commits must chain correctly. |
| **Signature** | If commit has `signature`, verifies ed25519 signature against `signer_key`. |
| **Hash** | Warns if computed hash doesn't match (for debugging). |
| **Structure** | Requires `hash` and `data` fields. |

### Validation Errors

```json
{
  "error": "Commit validation failed",
  "validation_errors": [
    "commits[0]: parent 'abc123' not found in contract history",
    "commits[1]: signature verification failed"
  ]
}
```

### Signed Commits

To sign a commit:
```javascript
const message = `${commit.hash}:${commit.parent || ''}:${JSON.stringify(commit.data)}`;
const signature = ed25519.sign(message, privateKey);

commit.signature = {
  signature: hex(signature),
  signer_key: hex(publicKey)
};
```

## Security Model

| Concern | Solution |
|---------|----------|
| Long-term key compromise | Identity key rarely used, only for access key management |
| Session key compromise | Revoke access key, create new one |
| Replay attacks | Timestamp checking (5 min window) |
| Man-in-the-middle | All requests signed |
| Access control | Permissions granted to identities, not access keys |
| Invalid commits | Validated on push (parent chain, signatures) |

## Data Storage

SQLite database with tables:
- `identities` - Long-term identity keys
- `access` - Session access keys (linked to identity)
- `contracts` - Contract metadata (owned by identity)
- `contract_access` - Access control (granted to identity)
- `commits` - Contract commits
