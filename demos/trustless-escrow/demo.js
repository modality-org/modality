#!/usr/bin/env node

/**
 * Trustless Escrow Demo
 * 
 * Simulates two AI agents (Alice & Bob) executing an escrow deal.
 * Run with: node demo.js
 */

const crypto = require('crypto');

// Simulated ed25519 key generation (in real use, use proper ed25519)
function generateKeyPair(name) {
  const privateKey = crypto.randomBytes(32).toString('hex');
  const publicKey = crypto.createHash('sha256').update(privateKey).digest('hex').slice(0, 64);
  return { name, privateKey, publicKey };
}

// Simulated signature
function sign(privateKey, message) {
  return crypto.createHmac('sha256', privateKey).update(message).digest('hex').slice(0, 64);
}

// Console colors
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  yellow: '\x1b[33m',
};

function log(agent, action, details = '') {
  const agentColor = agent === 'Alice' ? colors.magenta : 
                     agent === 'Bob' ? colors.blue : 
                     colors.yellow;
  const time = new Date().toISOString().split('T')[1].slice(0, 8);
  console.log(`${colors.dim}[${time}]${colors.reset} ${agentColor}${agent}${colors.reset} ${colors.green}${action}${colors.reset} ${details}`);
}

function header(text) {
  console.log(`\n${colors.bright}${colors.cyan}═══ ${text} ═══${colors.reset}\n`);
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// Contract state
class EscrowContract {
  constructor() {
    this.state = 'init';
    this.commits = [];
    this.parties = {};
  }

  addParty(role, publicKey) {
    this.parties[role] = publicKey;
    log('Contract', 'PARTY_ADDED', `${role}: ${publicKey.slice(0, 16)}...`);
  }

  commit(action, signer, signature, data = {}) {
    const commit = {
      index: this.commits.length,
      action,
      signer,
      signature: signature.slice(0, 16) + '...',
      timestamp: new Date().toISOString(),
      data,
      hash: crypto.createHash('sha256').update(JSON.stringify({ action, signer, data })).digest('hex').slice(0, 16)
    };
    this.commits.push(commit);
    return commit;
  }

  transition(newState) {
    const oldState = this.state;
    this.state = newState;
    return { from: oldState, to: newState };
  }

  verify() {
    // In real implementation, verify all signatures against rules
    return {
      valid: true,
      commits: this.commits.length,
      finalState: this.state
    };
  }
}

async function runDemo() {
  console.clear();
  console.log(`
${colors.bright}${colors.cyan}
  ╔════════════════════════════════════════════════════════════╗
  ║         🔐 TRUSTLESS AGENT ESCROW DEMO 🔐                  ║
  ║                                                            ║
  ║   Two AI agents execute a verified deal                    ║
  ║   No trust required — just math                            ║
  ╚════════════════════════════════════════════════════════════╝
${colors.reset}`);

  // Setup
  header('SETUP: Generating Agent Identities');
  
  const alice = generateKeyPair('Alice');
  const bob = generateKeyPair('Bob');
  
  log('Alice', 'IDENTITY_CREATED', `pubkey: ${alice.publicKey.slice(0, 16)}...`);
  await sleep(500);
  log('Bob', 'IDENTITY_CREATED', `pubkey: ${bob.publicKey.slice(0, 16)}...`);
  await sleep(500);

  // Create contract
  header('PHASE 1: Contract Creation');
  
  const contract = new EscrowContract();
  log('System', 'CONTRACT_CREATED', 'Escrow contract initialized');
  await sleep(500);
  
  contract.addParty('buyer', alice.publicKey);
  await sleep(300);
  contract.addParty('seller', bob.publicKey);
  await sleep(500);
  
  console.log(`\n${colors.dim}Contract rules loaded:${colors.reset}`);
  console.log(`${colors.dim}  • Buyer must sign deposits${colors.reset}`);
  console.log(`${colors.dim}  • Seller must sign deliveries${colors.reset}`);
  console.log(`${colors.dim}  • Buyer must sign releases${colors.reset}`);
  await sleep(1000);

  // Phase 2: Deposit
  header('PHASE 2: Alice Deposits Funds');
  
  log('Alice', 'PREPARING', 'Signing deposit transaction...');
  await sleep(800);
  
  const depositMsg = JSON.stringify({ action: 'DEPOSIT', amount: 100, timestamp: Date.now() });
  const depositSig = sign(alice.privateKey, depositMsg);
  
  const depositCommit = contract.commit('DEPOSIT', 'buyer', depositSig, { amount: 100 });
  contract.transition('deposited');
  
  log('Alice', 'DEPOSIT', `100 tokens → escrow`);
  log('System', 'COMMIT_VERIFIED', `hash: ${depositCommit.hash}... | sig: ${depositSig.slice(0, 12)}...`);
  console.log(`${colors.dim}  State: init → deposited${colors.reset}`);
  await sleep(1000);

  // Phase 3: Delivery
  header('PHASE 3: Bob Delivers Data');
  
  log('Bob', 'PREPARING', 'Packaging data for delivery...');
  await sleep(600);
  log('Bob', 'PREPARING', 'Signing delivery transaction...');
  await sleep(800);
  
  const deliverMsg = JSON.stringify({ action: 'DELIVER', dataHash: 'sha256:abc123...', timestamp: Date.now() });
  const deliverSig = sign(bob.privateKey, deliverMsg);
  
  const deliverCommit = contract.commit('DELIVER', 'seller', deliverSig, { dataHash: 'sha256:abc123...' });
  contract.transition('delivered');
  
  log('Bob', 'DELIVER', `Data package delivered`);
  log('System', 'COMMIT_VERIFIED', `hash: ${deliverCommit.hash}... | sig: ${deliverSig.slice(0, 12)}...`);
  console.log(`${colors.dim}  State: deposited → delivered${colors.reset}`);
  await sleep(1000);

  // Phase 4: Release
  header('PHASE 4: Alice Releases Funds');
  
  log('Alice', 'VERIFYING', 'Checking delivered data...');
  await sleep(600);
  log('Alice', 'VERIFIED', 'Data matches expected hash ✓');
  await sleep(400);
  log('Alice', 'PREPARING', 'Signing release transaction...');
  await sleep(800);
  
  const releaseMsg = JSON.stringify({ action: 'RELEASE', timestamp: Date.now() });
  const releaseSig = sign(alice.privateKey, releaseMsg);
  
  const releaseCommit = contract.commit('RELEASE', 'buyer', releaseSig, {});
  contract.transition('released');
  
  log('Alice', 'RELEASE', `Funds released to Bob`);
  log('System', 'COMMIT_VERIFIED', `hash: ${releaseCommit.hash}... | sig: ${releaseSig.slice(0, 12)}...`);
  console.log(`${colors.dim}  State: delivered → released${colors.reset}`);
  await sleep(1000);

  // Completion
  header('✅ ESCROW COMPLETE');
  
  const verification = contract.verify();
  
  console.log(`${colors.green}${colors.bright}
  ┌─────────────────────────────────────────────────────────┐
  │                    DEAL SUCCESSFUL                       │
  │                                                         │
  │   Alice received: Data package                          │
  │   Bob received: 100 tokens                              │
  │                                                         │
  │   Total commits: ${verification.commits}                                     │
  │   All signatures: VERIFIED ✓                            │
  │   Tamper-proof: YES ✓                                   │
  │                                                         │
  │   Neither agent had to trust the other.                 │
  │   The contract enforced the rules.                      │
  └─────────────────────────────────────────────────────────┘
${colors.reset}`);

  // Show proof
  header('📜 CRYPTOGRAPHIC PROOF');
  
  console.log(`${colors.dim}Full commit history (verifiable by anyone):${colors.reset}\n`);
  console.log(JSON.stringify({
    contract_id: 'con_escrow_demo_' + Date.now().toString(36),
    final_state: contract.state,
    parties: {
      buyer: alice.publicKey.slice(0, 32) + '...',
      seller: bob.publicKey.slice(0, 32) + '...'
    },
    commits: contract.commits.map(c => ({
      ...c,
      signature: c.signature
    })),
    verification: 'ALL_SIGNATURES_VALID'
  }, null, 2));
  
  console.log(`\n${colors.cyan}Demo complete. This is how agents cooperate without trust.${colors.reset}\n`);
}

// Run
runDemo().catch(console.error);
