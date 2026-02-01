#!/usr/bin/env node

/**
 * Example usage of Contract Hub client
 * 
 * Run the server first: npm start
 * Then: node example.js
 */

import { ContractHubClient } from './src/client.js';

const HUB_URL = process.env.HUB_URL || 'http://localhost:3100';

async function main() {
  console.log('=== Contract Hub Example ===\n');
  
  // 1. Generate keypair
  console.log('1. Generating keypair...');
  const { privateKey, publicKey } = await ContractHubClient.generateKeypair();
  console.log('   Public key:', publicKey.slice(0, 16) + '...');
  
  // 2. Create client and register
  console.log('\n2. Registering with hub...');
  const client = new ContractHubClient(HUB_URL);
  const { access_id } = await client.register(publicKey);
  console.log('   Access ID:', access_id);
  
  // 3. Configure client with credentials
  client.accessId = access_id;
  client.privateKey = privateKey;
  
  // 4. Create a contract
  console.log('\n3. Creating contract...');
  const { contract_id } = await client.createContract(
    'Example Contract',
    'A demonstration contract'
  );
  console.log('   Contract ID:', contract_id);
  
  // 5. Push some commits
  console.log('\n4. Pushing commits...');
  const commits = [
    {
      hash: 'commit_001',
      data: {
        type: 'POST',
        path: '/users/alice.id',
        content: 'abc123pubkey'
      },
      parent: null
    },
    {
      hash: 'commit_002',
      data: {
        type: 'RULE',
        path: '/rules/auth.modality',
        content: 'always([+claim] implies signed_by(/users/alice.id))'
      },
      parent: 'commit_001'
    }
  ];
  
  const pushResult = await client.push(contract_id, commits);
  console.log('   Pushed:', pushResult.pushed, 'commits');
  console.log('   Head:', pushResult.head);
  
  // 6. Pull commits
  console.log('\n5. Pulling commits...');
  const pullResult = await client.pull(contract_id);
  console.log('   Head:', pullResult.head);
  console.log('   Commits:', pullResult.commits.length);
  for (const c of pullResult.commits) {
    console.log('    -', c.hash, ':', c.data?.type || 'data');
  }
  
  // 7. Get contract info
  console.log('\n6. Getting contract info...');
  const info = await client.getContract(contract_id);
  console.log('   Name:', info.name);
  console.log('   Head:', info.head);
  console.log('   Owner:', info.owner);
  
  // 8. List contracts
  console.log('\n7. Listing all contracts...');
  const { contracts } = await client.listContracts();
  console.log('   Found:', contracts.length, 'contracts');
  
  console.log('\n=== Example Complete ===');
  console.log('\nTo grant access to another user:');
  console.log(`  await client.grantAccess('${contract_id}', 'acc_OTHER_ID', 'read');`);
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
