/**
 * Bank Deposits Example
 * 
 * Multi-account bank on a single contract:
 * - Admin registers accounts
 * - Each account has its own balance
 * - Withdrawals checked against balance via balance_sufficient predicate
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
  
  console.log('=== Multi-Account Bank Example ===\n');
  
  // Create bank contract
  const bank = Contract.create();
  await bank.init();
  
  // === Setup ===
  console.log('--- Bank Setup ---');
  
  // Admin setup
  await bank.post('/bank/admin.id', admin.publicKeyHex, admin);
  await bank.post('/bank/name.text', 'Modal Bank', admin);
  
  // Add model
  await bank.post('/bank/model.modality', `
    model bank {
      initial open
      open -> open [+REGISTER_ACCOUNT +signed_by(/bank/admin.id)]
      open -> open [+DEPOSIT +signed_by(/action/account.id)]
      open -> open [+WITHDRAW]
      open -> paused [+PAUSE +signed_by(/bank/admin.id)]
      paused -> open [+RESUME +signed_by(/bank/admin.id)]
    }
  `, admin);
  
  // Add withdrawal rule using balance_sufficient predicate
  await bank.addRule(`
    rule withdrawal_limit {
      starting_at $PARENT
      formula {
        always (
          [+WITHDRAW] implies (
            signed_by(/action/account.id) &
            balance_sufficient(
              /bank/accounts/{/action/account_id}.json:balance,
              /action/amount
            )
          )
        )
      }
    }
  `, admin);
  
  console.log('Bank initialized with withdrawal rules\n');
  
  // === Register Accounts ===
  console.log('--- Registering Accounts ---');
  
  // Admin registers accounts
  await bank.post('/bank/accounts/alice.json', {
    id: alice.publicKeyHex,
    balance: 0,
    created: Date.now()
  }, admin);
  await bank.doAction('REGISTER_ACCOUNT', { account_id: 'alice' }, admin);
  console.log('Registered: alice');
  
  await bank.post('/bank/accounts/bob.json', {
    id: bob.publicKeyHex,
    balance: 0,
    created: Date.now()
  }, admin);
  await bank.doAction('REGISTER_ACCOUNT', { account_id: 'bob' }, admin);
  console.log('Registered: bob');
  
  await bank.post('/bank/accounts/charlie.json', {
    id: charlie.publicKeyHex,
    balance: 0,
    created: Date.now()
  }, admin);
  await bank.doAction('REGISTER_ACCOUNT', { account_id: 'charlie' }, admin);
  console.log('Registered: charlie\n');
  
  // === Deposits ===
  console.log('--- Deposits ---');
  
  // Alice deposits 500
  let aliceAccount = bank.get('/bank/accounts/alice.json');
  await bank.post('/bank/accounts/alice.json', {
    ...aliceAccount,
    balance: aliceAccount.balance + 500
  }, alice);
  await bank.doAction('DEPOSIT', { 
    account_id: 'alice',
    amount: 500 
  }, alice);
  console.log('Alice deposited 500');
  
  // Bob deposits 1000
  let bobAccount = bank.get('/bank/accounts/bob.json');
  await bank.post('/bank/accounts/bob.json', {
    ...bobAccount,
    balance: bobAccount.balance + 1000
  }, bob);
  await bank.doAction('DEPOSIT', { 
    account_id: 'bob',
    amount: 1000 
  }, bob);
  console.log('Bob deposited 1000');
  
  // Charlie deposits 250
  let charlieAccount = bank.get('/bank/accounts/charlie.json');
  await bank.post('/bank/accounts/charlie.json', {
    ...charlieAccount,
    balance: charlieAccount.balance + 250
  }, charlie);
  await bank.doAction('DEPOSIT', { 
    account_id: 'charlie',
    amount: 250 
  }, charlie);
  console.log('Charlie deposited 250\n');
  
  // === Check Balances ===
  console.log('--- Current Balances ---');
  console.log('Alice:', bank.get('/bank/accounts/alice.json').balance);
  console.log('Bob:', bank.get('/bank/accounts/bob.json').balance);
  console.log('Charlie:', bank.get('/bank/accounts/charlie.json').balance);
  console.log('');
  
  // === Withdrawals ===
  console.log('--- Withdrawals ---');
  
  // Alice withdraws 200 (valid: 200 <= 500)
  aliceAccount = bank.get('/bank/accounts/alice.json');
  await bank.post('/bank/accounts/alice.json', {
    ...aliceAccount,
    balance: aliceAccount.balance - 200
  }, alice);
  await bank.doAction('WITHDRAW', { 
    account_id: 'alice',
    amount: 200 
  }, alice);
  console.log('Alice withdrew 200 ✓');
  
  // Bob withdraws 300 (valid: 300 <= 1000)
  bobAccount = bank.get('/bank/accounts/bob.json');
  await bank.post('/bank/accounts/bob.json', {
    ...bobAccount,
    balance: bobAccount.balance - 300
  }, bob);
  await bank.doAction('WITHDRAW', { 
    account_id: 'bob',
    amount: 300 
  }, bob);
  console.log('Bob withdrew 300 ✓');
  
  // Show what would happen with invalid withdrawal
  console.log('\n[Simulated] Charlie tries to withdraw 500 (invalid: 500 > 250)');
  console.log('  → Hub would reject: balance_sufficient(250, 500) = false\n');
  
  // === Final Balances ===
  console.log('--- Final Balances ---');
  console.log('Alice:', bank.get('/bank/accounts/alice.json').balance);
  console.log('Bob:', bank.get('/bank/accounts/bob.json').balance);
  console.log('Charlie:', bank.get('/bank/accounts/charlie.json').balance);
  
  // === Summary ===
  console.log('\n--- Contract Summary ---');
  console.log('Commits:', bank.commits.length);
  console.log('Accounts:', ['alice', 'bob', 'charlie'].length);
}

main().catch(console.error);
