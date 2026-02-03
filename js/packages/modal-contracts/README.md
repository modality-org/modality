# @modality-dev/modal-contracts

High-level JavaScript API for creating and interacting with Modal Contracts.

## Installation

```bash
npm install @modality-dev/modal-contracts
```

## Quick Start

```javascript
import { Contract, Identity, wasm } from '@modality-dev/modal-contracts';

// Initialize WASM (required for verification)
await wasm.init();

// Create identities
const alice = await Identity.generate();
const bob = await Identity.generate();

// Create a contract
const contract = Contract.create();
await contract.init();

// Add parties
await contract.post('/parties/alice.id', alice.publicKeyHex, alice);
await contract.post('/parties/bob.id', bob.publicKeyHex, alice);

// Add a model
await contract.post('/model.modality', `
  model escrow {
    states { idle, funded, complete }
    initial { idle }
    transitions {
      idle -[DEPOSIT]-> funded
      funded -[RELEASE]-> complete
    }
  }
`, alice);

// Add a rule
await contract.addRule(`
  export default rule {
    starting_at $PARENT
    formula {
      always ([<+RELEASE>] signed_by(/parties/bob.id))
    }
  }
`, alice);

// Perform an action
await contract.doAction('DEPOSIT', { amount: 100 }, alice);

// Generate diagram
console.log(contract.mermaid());
```

## API

### Contract

```javascript
import { Contract } from '@modality-dev/modal-contracts';

// Create a new contract
const contract = Contract.create();
await contract.init(); // Initialize WASM

// State operations
contract.get('/path');     // Get value at path
contract.has('/path');     // Check if path exists
contract.paths();          // Get all paths
contract.state();          // Get full state object

// Add commits
await contract.post(path, value, signer);      // POST: set value
await contract.addRule(ruleContent, signer);   // RULE: add rule
await contract.doAction(action, params, signer); // ACTION: perform action

// Model operations
contract.model();          // Get parsed model
contract.mermaid();        // Generate Mermaid diagram
contract.checkFormula(f);  // Check formula against model

// Serialization
const json = contract.toJSON();
const restored = Contract.fromJSON(json);
```

### Identity

```javascript
import { Identity } from '@modality-dev/modal-contracts';

// Generate new keypair
const identity = await Identity.generate();

// From existing keys
const fromPrivate = await Identity.fromPrivateKey(hexPrivateKey);
const fromPublic = Identity.fromPublicKey(hexPublicKey);

// Properties
identity.publicKeyHex;   // 64-char hex public key
identity.privateKeyHex;  // 64-char hex private key (if available)
identity.canSign();      // true if has private key

// Sign and verify
const signature = await identity.signHex(message);
const valid = await identity.verify(message, signature);
```

### Commit

```javascript
import { Commit, CommitType } from '@modality-dev/modal-contracts';

// Create commits
const post = Commit.post(parent, '/path', value);
const rule = Commit.rule(parent, ruleContent);
const action = Commit.action(parent, 'ACTION', { params });
const del = Commit.delete(parent, '/path');

// Sign
await commit.sign(identity);
commit.isSignedBy(publicKeyHex);

// Properties
commit.hash();           // SHA256 hash
commit.parent;           // Parent commit hash
commit.type;             // POST, RULE, ACTION, DELETE
commit.signatures;       // Array of {publicKey, signature}
```

### PathValue

```javascript
import { PathValue, PathType } from '@modality-dev/modal-contracts';

// Create typed values
const flag = PathValue.bool('/config/enabled', true);
const name = PathValue.text('/info/name', 'Alice');
const data = PathValue.json('/data/info', { foo: 'bar' });
const id = PathValue.id('/parties/alice', publicKeyHex);
const model = PathValue.modality('/model', modelContent);

// Path types
PathType.BOOL;      // .bool
PathType.TEXT;      // .text
PathType.JSON;      // .json
PathType.ID;        // .id
PathType.MODALITY;  // .modality
```

### ContractStore

```javascript
import { ContractStore } from '@modality-dev/modal-contracts';

const store = new ContractStore();

// CRUD operations
const contract = store.create();  // Create new contract
store.add(existingContract);      // Add existing
store.get(id);                    // Get by ID
store.has(id);                    // Check existence
store.remove(id);                 // Remove
store.list();                     // List IDs
store.all();                      // Get all contracts

// Serialization
const json = store.toJSON();
const restored = ContractStore.fromJSON(json);
```

## WASM Functions

Direct access to Rust-powered parsing and verification:

```javascript
import { wasm } from '@modality-dev/modal-contracts';

await wasm.init();

// Parsing
const model = wasm.parseModel(content);
const models = wasm.parseAllModels(content);
const formulas = wasm.parseFormulas(content);

// Mermaid diagrams
const diagram = wasm.generateMermaid(model);
const styled = wasm.generateMermaidStyled(model);
const withState = wasm.generateMermaidWithState(model);

// Model checking
const result = wasm.checkFormula(model, formula);
const anyState = wasm.checkFormulaAnyState(model, formula);
```

## Examples

### Escrow Contract

```javascript
// Setup
const buyer = await Identity.generate();
const seller = await Identity.generate();
const contract = await Contract.create().init();

// Set up parties
await contract.post('/parties/buyer.id', buyer.publicKeyHex, buyer);
await contract.post('/parties/seller.id', seller.publicKeyHex, buyer);

// Add escrow model
await contract.post('/model.modality', `
  model escrow {
    states { idle, funded, released, refunded }
    initial { idle }
    transitions {
      idle -[DEPOSIT]-> funded
      funded -[RELEASE]-> released
      funded -[REFUND]-> refunded
    }
  }
`, buyer);

// Add protection rules
await contract.addRule(`
  rule {
    starting_at $PARENT
    formula {
      always ([<+RELEASE>] signed_by(/parties/seller.id))
    }
  }
`, buyer);

// Buyer deposits
await contract.doAction('DEPOSIT', { amount: 1000 }, buyer);

// Seller releases (only they can)
await contract.doAction('RELEASE', {}, seller);
```

## Building

```bash
# Build WASM
cd js/packages/wasm
npm run build

# Test
npm test
```

## Links

- [Modality Documentation](https://docs.modality.org)
- [Modal Money](https://modal.money)
- [GitHub](https://github.com/modality-org/modality)
