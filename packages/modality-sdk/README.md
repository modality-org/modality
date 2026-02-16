# @modality-org/sdk

SDK for creating, signing, and verifying Modality contracts. Designed for AI agents using the Agent Trust Protocol (ATP).

## Install

```bash
npm install @modality-org/sdk
```

## Usage

### Identity (Ed25519 keys)

```ts
import { generateIdentity, signJSON, verifyJSON } from '@modality-org/sdk';

const alice = generateIdentity();
const sig = signJSON({ hello: 'world' }, alice.privateKey);
const ok = verifyJSON({ hello: 'world' }, sig, alice.publicKey); // true
```

### Create a Contract from an Intent Template

```ts
import { generateIdentity, escrow, createContract, verifyGenesis } from '@modality-org/sdk';

const buyer = generateIdentity();
const seller = generateIdentity();

const intent = escrow(
  { buyer: buyer.publicKey, seller: seller.publicKey },
  { amount: 100, currency: 'USDC', delivery_deadline: '2025-03-01T00:00:00Z', dispute_window_hours: 24 },
);

const genesis = createContract(intent, buyer);
console.log(genesis.contract_id);
console.log(verifyGenesis(genesis)); // true
```

### Generate a Contract Card

```ts
import { generateCard } from '@modality-org/sdk';

const card = generateCard(genesis, seller.publicKey);
console.log(card.my_role);        // 'seller'
console.log(card.my_rights);      // ['Receive payment upon delivery confirmation', ...]
console.log(card.available_actions); // ['DELIVER']
```

### Commits

```ts
import { createCommit, signCommit, verifyCommit, commitHash } from '@modality-org/sdk';

let commit = createCommit(genesis.contract_id, 1, [
  { method: 'post', path: '/escrow/deposit', value: { amount: 100 } },
], null);

commit = signCommit(commit, buyer);
console.log(verifyCommit(commit)); // true
console.log(commitHash(commit));
```

### Task Delegation

```ts
import { generateIdentity, taskDelegation, createContract, generateCard } from '@modality-org/sdk';

const delegator = generateIdentity();
const worker = generateIdentity();

const intent = taskDelegation(
  { delegator: delegator.publicKey, worker: worker.publicKey },
  { task: 'Summarize 10 documents', payment: 50, deadline: '2025-03-01T12:00:00Z' },
);

const genesis = createContract(intent, delegator);
const workerCard = generateCard(genesis, worker.publicKey);
// workerCard.available_actions => ['ACCEPT', 'REJECT']
```

## API

### Identity
- `generateIdentity()` — new Ed25519 keypair
- `identityFromPrivateKey(hex)` — restore from private key
- `sign(message, privateKey)` / `verify(message, sig, publicKey)`
- `signJSON(data, privateKey)` / `verifyJSON(data, sig, publicKey)`

### Contracts
- `createContract(intent, creator)` — create and sign a genesis
- `verifyGenesis(genesis)` — verify genesis signature

### Commits
- `createCommit(contractId, seq, actions, parentHash)`
- `signCommit(commit, identity)` / `verifyCommit(commit)`
- `commitHash(commit)`

### Contract Cards
- `generateCard(genesis, myPublicKey)` — ATP Contract Card for a party

### Intent Templates
- `escrow(parties, terms, options?)`
- `taskDelegation(parties, terms, options?)`
- `dataExchange(parties, terms, options?)`

## License

MIT
