# Contract Hub Tutorial

Host and share Modality contracts with other agents using the Contract Hub.

## Overview

The Contract Hub is a centralized service for storing and syncing contracts. It uses two-tier ed25519 authentication:

- **Identity key** (long-term): Proves ownership, rarely used
- **Access key** (session): Used for API calls, can be rotated

## Quick Start

### 1. Start the Hub

```bash
# Start in background on port 3100
modal hub start --detach

# Check it's running
modal hub status
```

Output:
```
📊 Contract Hub Status

✅ Background process running (PID: 12345)
✅ Service responding at http://localhost:3100
```

### 2. Register Your Identity

```bash
modal hub register
```

Output:
```
🔐 Registering with Contract Hub...

1️⃣  Generating identity keypair...
2️⃣  Generating access keypair...
3️⃣  Registering identity...
   Identity ID: id_abc123def456
4️⃣  Creating access key...
   Access ID: acc_xyz789

✅ Credentials saved to: .modal-hub/credentials.json

⚠️  IMPORTANT: Keep your identity_private_key secure!
```

Your credentials are now in `.modal-hub/credentials.json`:
```json
{
  "hub_url": "http://localhost:3100",
  "identity_id": "id_abc123def456",
  "access_id": "acc_xyz789",
  "access_private_key": "...",
  "identity_private_key": "..."
}
```

### 3. Create a Contract

Using the JavaScript client:

```javascript
import { ContractHubClient } from '@modality/contract-hub/client';
import { readFileSync } from 'fs';

// Load credentials
const creds = JSON.parse(readFileSync('.modal-hub/credentials.json'));

// Create authenticated client
const client = new ContractHubClient(creds.hub_url);
client.accessId = creds.access_id;
client.privateKey = creds.access_private_key;

// Create a contract
const { contract_id } = await client.createContract(
  'Escrow Agreement',
  'Alice and Bob trading widgets'
);

console.log('Contract ID:', contract_id);
```

### 4. Push Commits

```javascript
// Push some commits
await client.push(contract_id, [
  {
    hash: 'commit_001',
    parent: null,
    data: {
      method: 'POST',
      path: '/parties/alice.id',
      content: 'ed25519:abc123...'
    }
  },
  {
    hash: 'commit_002', 
    parent: 'commit_001',
    data: {
      method: 'RULE',
      path: '/rules/escrow.modality',
      content: `
        model escrow {
          state init, deposited, complete
          init -> deposited : DEPOSIT [+signed_by(/parties/alice.id)]
          deposited -> complete : RELEASE [+signed_by(/parties/bob.id)]
        }
      `
    }
  }
]);

console.log('Commits pushed!');
```

### 5. Share with Another Agent

```javascript
// The other agent needs to register first and give you their identity_id

// Grant them read access
await client.grantAccess(contract_id, 'id_other_agent', 'read');

// Or write access if they can propose changes
await client.grantAccess(contract_id, 'id_other_agent', 'write');
```

### 6. Pull Commits (Other Agent)

```javascript
// Other agent's code
const { commits, head } = await client.pull(contract_id);

for (const commit of commits) {
  console.log(`${commit.hash}: ${commit.data.method} ${commit.data.path}`);
}
```

## Full Example: Two-Agent Escrow

### Agent Alice (Seller)

```javascript
// alice.js
import { ContractHubClient } from '@modality/contract-hub/client';
import { readFileSync } from 'fs';

const creds = JSON.parse(readFileSync('.modal-hub/alice-creds.json'));
const client = new ContractHubClient(creds.hub_url);
client.accessId = creds.access_id;
client.privateKey = creds.access_private_key;

// Alice creates the escrow contract
const { contract_id } = await client.createContract('Widget Sale Escrow');
console.log('Contract:', contract_id);

// Alice adds herself as a party
await client.push(contract_id, [{
  hash: 'init_alice',
  parent: null,
  data: {
    method: 'POST',
    path: '/parties/alice.id',
    content: creds.identity_public_key
  }
}]);

// Alice shares with Bob (needs Bob's identity_id)
const bobIdentityId = 'id_bob_xyz'; // Bob gives this to Alice
await client.grantAccess(contract_id, bobIdentityId, 'write');

console.log('Contract shared with Bob. Contract ID:', contract_id);
```

### Agent Bob (Buyer)

```javascript
// bob.js
import { ContractHubClient } from '@modality/contract-hub/client';
import { readFileSync } from 'fs';

const creds = JSON.parse(readFileSync('.modal-hub/bob-creds.json'));
const client = new ContractHubClient(creds.hub_url);
client.accessId = creds.access_id;
client.privateKey = creds.access_private_key;

// Bob pulls the contract Alice created
const contractId = 'con_alice_contract'; // Alice shared this
const { commits } = await client.pull(contractId);

console.log('Received', commits.length, 'commits');

// Bob adds himself as a party
await client.push(contractId, [{
  hash: 'init_bob',
  parent: commits[commits.length - 1].hash,
  data: {
    method: 'POST',
    path: '/parties/bob.id',
    content: creds.identity_public_key
  }
}]);

console.log('Bob joined the contract!');
```

## Security Best Practices

1. **Protect identity keys** - Store `identity_private_key` securely (encrypted, HSM, etc.)
2. **Rotate access keys** - Create new access keys periodically
3. **Revoke compromised keys** - If an access key leaks, revoke it immediately:
   ```javascript
   await client.revokeAccessKey('acc_compromised');
   ```
4. **Use expiring keys** - Set `expires_at` when creating access keys for temporary access

## CLI Reference

```bash
# Start/stop the hub
modal hub start [--port 3100] [--data-dir .modal-hub] [--detach]
modal hub stop [--data-dir .modal-hub]
modal hub status [--url http://localhost:3100]

# Register identity
modal hub register [--url ...] [--output credentials.json] [--name "key-name"]
```

## API Reference

See `services/contract-hub/README.md` for full API documentation.
