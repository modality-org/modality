# JavaScript SDK with Hub

Use the `@modality-dev/modal-contracts` package to create contracts and sync them with a hub.

## Installation

```bash
npm install @modality-dev/modal-contracts
```

## Quick Start

```javascript
import { 
  RemoteContract, 
  Identity, 
  HubClient 
} from '@modality-dev/modal-contracts';

// Create identity
const alice = await Identity.generate();

// Create contract connected to hub
const contract = new RemoteContract({
  id: 'my-contract',
  hubUrl: 'http://localhost:3000',
});
await contract.init();

// Add data
await contract.post('/info/name.text', 'Alice', alice);
await contract.post('/balance.json', { amount: 1000 }, alice);

// Push to hub
await contract.push();
console.log('Pushed to hub!');
```

## Hub Connection

### Direct Client

```javascript
import { HubClient } from '@modality-dev/modal-contracts';

const hub = new HubClient('http://localhost:3000');

// Check hub health
const health = await hub.health();
console.log(health); // { status: 'ok', version: '0.1.0' }

// Get contract info
const info = await hub.getContract('my-contract', {
  includeState: true,
});
console.log(info.state);
```

### With Contract

```javascript
import { RemoteContract } from '@modality-dev/modal-contracts';

// Option 1: URL in constructor
const contract = new RemoteContract({
  id: 'my-contract',
  hubUrl: 'http://localhost:3000',
});

// Option 2: Connect later
const contract = new RemoteContract({ id: 'my-contract' });
contract.connect('http://localhost:3000');

// Option 3: Load from hub
const contract = await RemoteContract.fromHub(
  'existing-contract',
  'http://localhost:3000'
);
```

## Creating Contracts

```javascript
import { RemoteContract, Identity } from '@modality-dev/modal-contracts';

const alice = await Identity.generate();
const bob = await Identity.generate();

const contract = new RemoteContract({
  id: `escrow-${Date.now()}`,
  hubUrl: 'http://localhost:3000',
});
await contract.init();

// Set up parties
await contract.post('/parties/alice.id', alice.publicKeyHex, alice);
await contract.post('/parties/bob.id', bob.publicKeyHex, alice);

// Add escrow model
await contract.post('/model.modality', `
  model escrow {
    states { idle, funded, released }
    initial { idle }
    transitions {
      idle -[DEPOSIT]-> funded
      funded -[RELEASE]-> released
    }
  }
`, alice);

// Add protection rule
await contract.addRule(`
  rule {
    starting_at $PARENT
    formula {
      always ([<+RELEASE>] signed_by(/parties/bob.id))
    }
  }
`, alice);

// Push to hub
const { pushed } = await contract.push();
console.log(`Pushed ${pushed} commits`);
```

## Syncing

### Push

```javascript
// Make local changes
await contract.post('/data/value.json', { key: 'value' }, alice);

// Check unpushed count
console.log(`${contract.unpushedCount()} commits to push`);

// Push to hub
const { pushed, head } = await contract.push();
console.log(`Pushed ${pushed} commits, head: ${head}`);
```

### Pull

```javascript
// Pull latest from hub
const { pulled, head } = await contract.pull();
console.log(`Pulled ${pulled} commits`);

// Access new state
console.log(contract.state());
```

### Sync (Pull + Push)

```javascript
// Sync bidirectionally
const { pulled, pushed, head } = await contract.sync();
console.log(`Pulled ${pulled}, pushed ${pushed}`);
```

## Multi-Party Example

Two parties collaborating on a contract:

```javascript
// Alice creates contract
const aliceId = await Identity.generate();
const contract = new RemoteContract({
  id: 'shared-contract',
  hubUrl: 'http://localhost:3000',
});
await contract.init();

await contract.post('/parties/alice.id', aliceId.publicKeyHex, aliceId);
await contract.push();

// Bob joins
const bobId = await Identity.generate();
const bobContract = await RemoteContract.fromHub(
  'shared-contract',
  'http://localhost:3000'
);

await bobContract.post('/parties/bob.id', bobId.publicKeyHex, bobId);
await bobContract.push();

// Alice pulls Bob's changes
await contract.pull();
console.log(contract.get('/parties/bob.id')); // Bob's public key
```

## Assets

Create and transfer assets:

```javascript
// Create asset
await contract.post('/assets/token.json', {
  method: 'create',
  asset_id: 'MY_TOKEN',
  quantity: 1000000,
  divisibility: 100,
}, alice);

// Send to another contract
await contract.post('/transfers/send-001.json', {
  method: 'send',
  asset_id: 'MY_TOKEN',
  to_contract: 'bob-contract',
  amount: 50000,
}, alice);

await contract.push();
```

## Error Handling

```javascript
import { HubError } from '@modality-dev/modal-contracts';

try {
  await contract.push();
} catch (error) {
  if (error instanceof HubError) {
    switch (error.code) {
      case 'TIMEOUT':
        console.error('Request timed out');
        break;
      case 'NOT_CONNECTED':
        console.error('Not connected to hub');
        break;
      case -32024:
        console.error('Insufficient balance');
        break;
      default:
        console.error(`Hub error: ${error.message}`);
    }
  } else {
    throw error;
  }
}
```

## API Reference

### HubClient

```javascript
const hub = new HubClient(url, { timeout: 30000 });

hub.health()                              // Check hub health
hub.version()                             // Get hub version
hub.getContract(id, { includeState })     // Get contract info
hub.getState(id)                          // Get contract state
hub.getCommits(id, { limit })             // List commits
hub.getCommit(id, hash)                   // Get specific commit
hub.submitCommit(id, commit)              // Submit single commit
hub.push(id, commits)                     // Push multiple commits
hub.pull(id, since)                       // Pull commits since hash
```

### RemoteContract

```javascript
const contract = new RemoteContract({ id, hubUrl });

contract.connect(url)        // Connect to hub
contract.isConnected()       // Check connection
contract.unpushedCount()     // Count unpushed commits

contract.pull()              // Pull from hub
contract.push()              // Push to hub  
contract.sync()              // Pull then push

// Inherited from Contract
contract.post(path, value, signer)
contract.addRule(content, signer)
contract.doAction(action, params, signer)
contract.get(path)
contract.state()
contract.model()
contract.mermaid()
```

## Browser Usage

```html
<!DOCTYPE html>
<script type="module">
import { RemoteContract, Identity } from '@modality-dev/modal-contracts';

async function main() {
  const id = await Identity.generate();
  const contract = new RemoteContract({
    id: 'browser-contract',
    hubUrl: 'http://localhost:3000',
  });
  await contract.init();
  
  await contract.post('/created.text', new Date().toISOString(), id);
  await contract.push();
  
  console.log('Contract created!', contract.id);
}

main();
</script>
```

## Next Steps

- [Commit Methods](/docs/reference/commit-methods) - All commit types
- [Assets Tutorial](/docs/tutorials/hub-and-assets) - CLI asset workflow
- [Modal Logic](/docs/concepts/modal-logic) - Rule formulas
