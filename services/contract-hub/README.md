# Contract Hub

Centralized HTTP service for push/pull of Modality contracts with ed25519 keypair authentication.

## Quick Start

```bash
# Install dependencies
npm install

# Start server
npm start

# Or with custom port/data dir
PORT=3100 DATA_DIR=./data npm start
```

## Authentication

All authenticated endpoints require three headers:

```
X-Access-Id: <your_access_id>
X-Timestamp: <unix_ms_timestamp>
X-Signature: <ed25519_signature>
```

The signature signs: `METHOD:PATH:TIMESTAMP:BODY_HASH`

### Generate Keypair

```javascript
import { ContractHubClient } from './src/client.js';

const { privateKey, publicKey } = await ContractHubClient.generateKeypair();
console.log('Private key (keep secret):', privateKey);
console.log('Public key:', publicKey);
```

### Register Access

```bash
curl -X POST http://localhost:3100/access/register \
  -H "Content-Type: application/json" \
  -d '{"public_key": "YOUR_PUBLIC_KEY_HEX"}'
```

Returns: `{ "access_id": "acc_xxx", "public_key": "..." }`

## API Endpoints

### No Auth Required

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/access/register` | Register public key |

### Auth Required

| Method | Path | Description |
|--------|------|-------------|
| POST | `/contracts` | Create contract |
| GET | `/contracts` | List your contracts |
| GET | `/contracts/:id` | Get contract info |
| POST | `/contracts/:id/push` | Push commits |
| GET | `/contracts/:id/pull` | Pull commits |
| GET | `/contracts/:id/commits/:hash` | Get specific commit |
| POST | `/contracts/:id/access` | Grant access |

## Client Example

```javascript
import { ContractHubClient } from './src/client.js';

// Generate or load keypair
const { privateKey, publicKey } = await ContractHubClient.generateKeypair();

// Create client (unauthenticated for registration)
const client = new ContractHubClient('http://localhost:3100');

// Register and get access ID
const { access_id } = await client.register(publicKey);
console.log('Access ID:', access_id);

// Configure client with credentials
client.accessId = access_id;
client.privateKey = privateKey;

// Create a contract
const { contract_id } = await client.createContract('My Contract', 'A test contract');
console.log('Contract ID:', contract_id);

// Push commits
await client.push(contract_id, [
  { hash: 'abc123', data: { message: 'Hello' }, parent: null }
]);

// Pull commits
const { commits } = await client.pull(contract_id);
console.log('Commits:', commits);

// Grant read access to another user
await client.grantAccess(contract_id, 'acc_other_user', 'read');
```

## Workflow

1. **Generate keypair** - Create ed25519 keypair for authentication
2. **Register** - Submit public key, receive access ID
3. **Create contract** - Initialize new contract storage
4. **Push** - Upload commits (signed changes)
5. **Pull** - Download commits (sync)
6. **Grant access** - Share with other users

## Data Storage

SQLite database in `DATA_DIR/contracts.db` with tables:
- `access` - Access keys
- `contracts` - Contract metadata
- `contract_access` - Access control
- `commits` - Contract commits

## Security

- Ed25519 signatures prevent unauthorized access
- Timestamp checking prevents replay attacks (5 min window)
- Access control per contract (owner, readers, writers)
- No password storage - pure keypair authentication
