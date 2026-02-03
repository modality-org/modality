/**
 * Bank Deposits Example
 * 
 * Demonstrates a bank contract where:
 * - Multiple parties deposit assets
 * - Each party can only withdraw up to their balance
 * - Admin can pause/resume the bank
 * 
 * Run: node examples/bank_deposits.js
 */

import { Contract, Identity, wasm } from '@modality-dev/modal-contracts';

async function main() {
  await wasm.init();
  
  // Create identities
  const admin = await Identity.generate();
  const alice = await Identity.generate();
  const bob = await Identity.generate();
  const charlie = await Identity.generate();
  
  console.log('=== Bank Deposits Example ===\n');
  console.log('Parties:');
  console.log('  Admin:', admin.publicKeyHex.slice(0, 16) + '...');
  console.log('  Alice:', alice.publicKeyHex.slice(0, 16) + '...');
  console.log('  Bob:', bob.publicKeyHex.slice(0, 16) + '...');
  console.log('  Charlie:', charlie.publicKeyHex.slice(0, 16) + '...');
  
  // Create bank contract
  const bank = Contract.create();
  await bank.init();
  
  // Setup bank
  console.log('\n--- Setting up bank ---');
  await bank.post('/bank/admin.id', admin.publicKeyHex, admin);
  await bank.post('/bank/name.text', 'Modal Bank', admin);
  
  // Register parties
  await bank.post('/bank/parties/alice.id', alice.publicKeyHex, admin);
  await bank.post('/bank/parties/bob.id', bob.publicKeyHex, admin);
  await bank.post('/bank/parties/charlie.id', charlie.publicKeyHex, admin);
  
  // Initialize balances
  await bank.post('/bank/balances/alice.json', { amount: 0 }, admin);
  await bank.post('/bank/balances/bob.json', { amount: 0 }, admin);
  await bank.post('/bank/balances/charlie.json', { amount: 0 }, admin);
  
  // Add bank model
  await bank.post('/bank/model.modality', `
    model bank {
      initial open
      open -> open [+DEPOSIT]
      open -> open [+WITHDRAW]
      open -> paused [+PAUSE +signed_by(/bank/admin.id)]
      paused -> open [+RESUME +signed_by(/bank/admin.id)]
    }
  `, admin);
  
  console.log('Bank initialized');
  
  // Deposits
  console.log('\n--- Deposits ---');
  
  // Alice deposits 500
  await bank.post('/bank/balances/alice.json', { amount: 500 }, alice);
  await bank.doAction('DEPOSIT', { depositor: 'alice', amount: 500 }, alice);
  console.log('Alice deposited 500');
  
  // Bob deposits 1000
  await bank.post('/bank/balances/bob.json', { amount: 1000 }, bob);
  await bank.doAction('DEPOSIT', { depositor: 'bob', amount: 1000 }, bob);
  console.log('Bob deposited 1000');
  
  // Charlie deposits 250
  await bank.post('/bank/balances/charlie.json', { amount: 250 }, charlie);
  await bank.doAction('DEPOSIT', { depositor: 'charlie', amount: 250 }, charlie);
  console.log('Charlie deposited 250');
  
  // Check balances
  console.log('\n--- Current Balances ---');
  console.log('Alice:', bank.get('/bank/balances/alice.json'));
  console.log('Bob:', bank.get('/bank/balances/bob.json'));
  console.log('Charlie:', bank.get('/bank/balances/charlie.json'));
  
  // Withdrawals
  console.log('\n--- Withdrawals ---');
  
  // Alice withdraws 200 (allowed: 200 <= 500)
  const aliceBalance = bank.get('/bank/balances/alice.json').amount;
  const aliceWithdraw = 200;
  await bank.post('/bank/balances/alice.json', { amount: aliceBalance - aliceWithdraw }, alice);
  await bank.doAction('WITHDRAW', { withdrawer: 'alice', amount: aliceWithdraw }, alice);
  console.log(`Alice withdrew ${aliceWithdraw} (remaining: ${aliceBalance - aliceWithdraw})`);
  
  // Bob withdraws 300 (allowed: 300 <= 1000)
  const bobBalance = bank.get('/bank/balances/bob.json').amount;
  const bobWithdraw = 300;
  await bank.post('/bank/balances/bob.json', { amount: bobBalance - bobWithdraw }, bob);
  await bank.doAction('WITHDRAW', { withdrawer: 'bob', amount: bobWithdraw }, bob);
  console.log(`Bob withdrew ${bobWithdraw} (remaining: ${bobBalance - bobWithdraw})`);
  
  // Final balances
  console.log('\n--- Final Balances ---');
  console.log('Alice:', bank.get('/bank/balances/alice.json'));
  console.log('Bob:', bank.get('/bank/balances/bob.json'));
  console.log('Charlie:', bank.get('/bank/balances/charlie.json'));
  
  // Show contract state
  console.log('\n--- Contract Summary ---');
  console.log('Total commits:', bank.commits.length);
  console.log('Head:', bank.head?.slice(0, 16) + '...');
  
  // Generate diagram
  const model = bank.model();
  if (model) {
    console.log('\n--- State Machine ---');
    console.log(wasm.generateMermaid(model));
  }
}

main().catch(console.error);
