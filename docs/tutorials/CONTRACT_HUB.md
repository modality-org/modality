# Contract Push/Pull Tutorial

Push and pull Modality contracts to a **hub** (centralized) or **chain** (decentralized).

## Overview

Modality contracts can sync to:
- **Hub** - Centralized HTTP service (fast, easy setup)
- **Chain** - Decentralized p2p network (trustless, validators)

Both use the same `modal c push` and `modal c pull` commands.

## Hub vs Chain

| Feature | Hub | Chain |
|---------|-----|-------|
| URL format | `http://...` | `/ip4/.../p2p/...` |
| Auth | ed25519 keypairs | Node identity |
| Validation | Server-side | Consensus |
| Speed | Fast | Depends on network |
| Trust | Hub operator | Validators |

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

## Full CLI Example: Two Agents Using `modal c`

The native `modal c push` and `modal c pull` commands work with the hub when you use an HTTP URL as the remote.

### Setup

```bash
# Start hub (one agent hosts it)
modal hub start --detach

# Each agent registers
modal hub register --output .modal-hub/credentials.json
```

### Alice: Create and Push Contract

```bash
# Create a new contract directory
mkdir my-escrow && cd my-escrow

# Initialize contract
modal c create --name "Widget Escrow"

# Add the hub as a remote
modal c set-remote origin http://localhost:3100

# Create some files
mkdir -p state rules

echo '{"seller": "alice", "buyer": "bob", "amount": 100}' > state/deal.json

echo 'model escrow {
  state init, deposited, released
  init -> deposited : DEPOSIT
  deposited -> released : RELEASE
}' > rules/escrow.modality

# Commit everything
modal c commit --all -m "Initial escrow setup"

# Push to hub
modal c push --remote origin
# Output: ✅ Successfully pushed 1 commit(s) to hub!
#         Contract ID: con_abc123
#         Remote: origin (http://localhost:3100)
```

### Bob: Clone and Contribute

```bash
# Create local contract directory
mkdir escrow-copy && cd escrow-copy

# Initialize with same contract ID
modal c create --contract-id con_abc123

# Add remote
modal c set-remote origin http://localhost:3100

# Pull Alice's commits
modal c pull --remote origin
# Output: ✅ Successfully pulled 1 commit(s)!

# Add Bob's signature
echo '{"signed_by": "bob", "timestamp": "2026-02-01"}' > state/bob-ack.json

# Commit and push
modal c commit --all -m "Bob acknowledges deal"
modal c push --remote origin
```

### Alice: Sync Bob's Changes

```bash
# Pull new commits
modal c pull --remote origin
# Output: ✅ Successfully pulled 1 commit(s)!
#         Pulled commits:
#           - 2d4e6f8a
```

## Alternative: Direct Hub Commands

You can also use the `modal hub` commands directly without the contract store:

### Alice creates and shares

```bash
# Create contract
modal hub create "Widget Escrow" --creds .modal-hub/credentials.json
# Output: ID: con_abc123

# Push a rule file
modal hub push con_abc123 --rule escrow.modality

# Share with Bob
modal hub grant con_abc123 id_bob_xyz write
```

### Bob pulls and contributes

```bash
# Pull
modal hub pull con_abc123

# Push data
modal hub push con_abc123 --file deal.json --path /state/deal.json
```

## Pushing to Chain (Decentralized)

Instead of a hub, push to chain validators for decentralized consensus.

### Setup

```bash
# Start a local node (or connect to existing network)
modal net start --config testnet

# Get your node's multiaddress
modal net info
# Output: /ip4/127.0.0.1/tcp/10101/p2p/12D3KooW...
```

### Add Chain Remote

```bash
cd my-contract

# Add chain as remote
modal c remote add chain /ip4/127.0.0.1/tcp/10101/p2p/12D3KooW...

# List remotes
modal c remote list
# Output:
#   hub (hub) -> http://localhost:3100
#   chain (chain) -> /ip4/127.0.0.1/tcp/10101/p2p/12D3KooW...
```

### Push to Chain

```bash
# Push to chain validators
modal c push --remote chain
# Output: ✅ Successfully pushed 2 commit(s)!
#         Contract ID: con_abc123
#         Remote: chain (/ip4/...)
```

### Pull from Chain

```bash
# Pull from chain
modal c pull --remote chain
```

## Multi-Remote Workflow

Use both hub and chain for redundancy:

```bash
# Add both remotes
modal c remote add hub http://hub.modality.network
modal c remote add chain /ip4/validator.modality.network/tcp/10101/p2p/...

# Push to hub first (fast)
modal c push --remote hub

# Then push to chain (consensus)
modal c push --remote chain

# Pull from either
modal c pull --remote hub
modal c pull --remote chain
```

## CLI Reference

```bash
# Remote management
modal c remote add <name> <url>    # Add remote (auto-detects hub/chain)
modal c remote remove <name>       # Remove remote
modal c remote list                # List all remotes
modal c remote get <name>          # Show remote URL

# Push/Pull (works with hub or chain)
modal c push [--remote <name>]     # Push to remote
modal c pull [--remote <name>]     # Pull from remote

# Hub server management
modal hub start [--port 3100] [--data-dir .modal-hub] [--detach]
modal hub stop [--data-dir .modal-hub]
modal hub status [--url http://localhost:3100]

# Hub identity
modal hub register [--url ...] [--output credentials.json]

# Direct hub commands (no contract store)
modal hub create <name>
modal hub push <contract> --file <file> [--path /path]
modal hub pull <contract>
modal hub grant <contract> <identity_id> [read|write]
```

## API Reference

See `services/contract-hub/README.md` for full API documentation.
