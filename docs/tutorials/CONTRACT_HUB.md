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

## Full CLI Example: Two Agents

### Setup (both agents)

```bash
# Start hub (only one agent needs to host)
modal hub start --detach

# Each agent registers
modal hub register --output alice-creds.json
modal hub register --output bob-creds.json
```

### Alice creates and shares

```bash
# Create contract
modal hub create "Widget Escrow" --creds alice-creds.json
# Output: ✅ Contract created
#         ID: con_abc123

# Add a rule
echo 'model escrow {
  state init, deposited, released
  init -> deposited : DEPOSIT
  deposited -> released : RELEASE
}' > escrow.modality

modal hub push con_abc123 --rule escrow.modality --creds alice-creds.json
# Output: ✅ Pushed 1 commit(s)
#         Head: 8f3a2b1c
#         8f3a2b1c RULE /rules/escrow.modality

# Share with Bob (Bob gives Alice his identity_id)
modal hub grant con_abc123 id_bob_xyz write --creds alice-creds.json
# Output: ✅ Granted write access
```

### Bob pulls and contributes

```bash
# Pull the contract
modal hub pull con_abc123 --creds bob-creds.json
# Output: 📥 Contract: con_abc123
#         Head: 8f3a2b1c
#         Commits: 1
#         8f3a2b1c RULE /rules/escrow.modality

# Add Bob's data
echo '{"amount": 100, "item": "widgets"}' > deal.json

modal hub push con_abc123 --file deal.json --path /state/deal.json --creds bob-creds.json
# Output: ✅ Pushed 1 commit(s)
#         Head: 2d4e6f8a

# List all contracts Bob has access to
modal hub list --creds bob-creds.json
```

### Alice syncs Bob's changes

```bash
# Pull latest (since Alice's last known head)
modal hub pull con_abc123 --since 8f3a2b1c --creds alice-creds.json
# Output: 📥 Contract: con_abc123
#         Head: 2d4e6f8a
#         Commits: 1
#         2d4e6f8a POST /state/deal.json

# Extract files locally
modal hub pull con_abc123 --output ./contract-files --creds alice-creds.json
# Output: ✅ Extracted 2 file(s) to ./contract-files
```

## CLI Reference

```bash
# Server management
modal hub start [--port 3100] [--data-dir .modal-hub] [--detach]
modal hub stop [--data-dir .modal-hub]
modal hub status [--url http://localhost:3100]

# Identity
modal hub register [--url ...] [--output credentials.json] [--name "key-name"]

# Contract operations
modal hub create <name> [--description "..."]
modal hub list [--json]
modal hub push <contract> --file <file> [--path /path/in/contract]
modal hub push <contract> --rule <rule.modality>
modal hub pull <contract> [--since <hash>] [--output <dir>] [--json]
modal hub grant <contract> <identity_id> [read|write]
```

## API Reference

See `services/contract-hub/README.md` for full API documentation.
