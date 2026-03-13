#!/usr/bin/env node

/**
 * Modality Tutorial — Multisig Transaction Authorization
 *
 * Five agents (Treasury, CFO, Board A, Board B, Mallory) demonstrate
 * threshold-based transaction authorization with real ed25519 signatures.
 *
 * Run: npm run demo
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils';

ed.etc.sha512Sync = (...m) => {
  const h = sha512.create();
  for (const msg of m) h.update(msg);
  return h.digest();
};

// ─── Colors ───────────────────────────────────────────────
const C = {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m',
  white: '\x1b[37m', bgRed: '\x1b[41m', bgGreen: '\x1b[42m',
};

// ─── Identity ─────────────────────────────────────────────
function generateIdentity(name) {
  const privateKey = ed.utils.randomPrivateKey();
  const publicKey = ed.getPublicKey(privateKey);
  return {
    name,
    privateKey: bytesToHex(privateKey),
    publicKey: bytesToHex(publicKey),
    shortKey: bytesToHex(publicKey).slice(0, 12) + '…',
  };
}

function signJSON(data, privateKeyHex) {
  const bytes = new TextEncoder().encode(JSON.stringify(data));
  return bytesToHex(ed.sign(bytes, hexToBytes(privateKeyHex)));
}

function verifyJSON(data, signatureHex, publicKeyHex) {
  const bytes = new TextEncoder().encode(JSON.stringify(data));
  return ed.verify(hexToBytes(signatureHex), bytes, hexToBytes(publicKeyHex));
}

function hashJSON(data) {
  return bytesToHex(sha256(new TextEncoder().encode(JSON.stringify(data))));
}

// ─── Multisig Transaction Engine ──────────────────────────

class MultisigContract {
  constructor(agents) {
    this.agents = agents; // { treasury, cfo, boardA, boardB }
    this.commits = [];

    // Valid signers by role
    this.roles = {
      treasury: agents.treasury.publicKey,
      cfo: agents.cfo.publicKey,
      boardA: agents.boardA.publicKey,
      boardB: agents.boardB.publicKey,
    };

    this.allValidKeys = new Set(Object.values(this.roles));
  }

  /**
   * Threshold rules:
   *   < $10K  → treasury alone (1-of-1)
   *   $10K–$100K → treasury + 1 board member (2-of-3: treasury required + 1 of {boardA, boardB, cfo})
   *   > $100K → treasury + cfo + 1 board member (3-of-4: treasury + cfo required + 1 of {boardA, boardB})
   */
  submit(description, amount, signers, data = {}) {
    const body = {
      description,
      amount,
      data,
      timestamp: new Date().toISOString(),
    };

    // 1. Check all signers are known
    for (const signer of signers) {
      if (!this.allValidKeys.has(signer.publicKey)) {
        return {
          ok: false,
          error: `Unknown signer '${signer.name}' — not an authorized agent`,
          amount,
          threshold: this._thresholdLabel(amount),
        };
      }
    }

    const signerKeys = new Set(signers.map(s => s.publicKey));
    const hasTreasury = signerKeys.has(this.roles.treasury);
    const hasCFO = signerKeys.has(this.roles.cfo);
    const boardCount = [this.roles.boardA, this.roles.boardB].filter(k => signerKeys.has(k)).length;

    // 2. Treasury must always sign
    if (!hasTreasury) {
      return {
        ok: false,
        error: `Treasury signature required for all transactions`,
        amount,
        threshold: this._thresholdLabel(amount),
      };
    }

    // 3. Threshold checks
    if (amount >= 100000) {
      // Large: treasury + cfo + 1 board
      if (!hasCFO) {
        return {
          ok: false,
          error: `Large transaction ($${amount.toLocaleString()}) requires CFO signature — missing`,
          amount,
          threshold: this._thresholdLabel(amount),
        };
      }
      if (boardCount < 1) {
        return {
          ok: false,
          error: `Large transaction ($${amount.toLocaleString()}) requires at least 1 board member — have ${boardCount}`,
          amount,
          threshold: this._thresholdLabel(amount),
        };
      }
    } else if (amount >= 10000) {
      // Medium: treasury + 1 board/cfo
      const coSigners = (hasCFO ? 1 : 0) + boardCount;
      if (coSigners < 1) {
        return {
          ok: false,
          error: `Medium transaction ($${amount.toLocaleString()}) requires 1 co-signer (board member or CFO) — have ${coSigners}`,
          amount,
          threshold: this._thresholdLabel(amount),
        };
      }
    }
    // Small: treasury alone is sufficient

    // 4. Build and sign commit
    const parentHash = this.commits.length > 0
      ? hashJSON(this.commits[this.commits.length - 1])
      : null;

    const signatures = {};
    for (const signer of signers) {
      signatures[signer.publicKey] = signJSON(body, signer.privateKey);
    }

    const commit = {
      sequence: this.commits.length,
      body,
      head: { parent_hash: parentHash, signatures },
    };

    // 5. Verify all signatures
    for (const [pubkey, sig] of Object.entries(signatures)) {
      if (!verifyJSON(body, sig, pubkey)) {
        return { ok: false, error: 'Signature verification failed', amount, threshold: this._thresholdLabel(amount) };
      }
    }

    this.commits.push(commit);
    return { ok: true, commit, amount, threshold: this._thresholdLabel(amount) };
  }

  _thresholdLabel(amount) {
    if (amount >= 100000) return '3-of-4 (treasury + CFO + board)';
    if (amount >= 10000) return '2-of-3 (treasury + co-signer)';
    return '1-of-1 (treasury)';
  }

  audit() {
    let valid = true;
    for (const commit of this.commits) {
      for (const [pubkey, sig] of Object.entries(commit.head.signatures)) {
        if (!verifyJSON(commit.body, sig, pubkey)) { valid = false; break; }
      }
    }
    return { valid, commitCount: this.commits.length };
  }
}

// ─── Display Helpers ──────────────────────────────────────

function printHeader(text) {
  const line = '═'.repeat(60);
  console.log(`\n${C.cyan}${C.bold}${line}${C.reset}`);
  console.log(`${C.cyan}${C.bold}  ${text}${C.reset}`);
  console.log(`${C.cyan}${C.bold}${line}${C.reset}\n`);
}

const agentColor = {
  'Treasury': C.blue,
  'CFO': C.magenta,
  'Board Member A': C.yellow,
  'Board Member B': C.green,
  'Mallory': C.red,
};

function printAgent(agent, action, detail = '') {
  const c = agentColor[agent.name] || C.white;
  console.log(`  ${c}${C.bold}${agent.name}${C.reset} ${C.green}→${C.reset} ${action}${detail ? ` ${C.dim}${detail}${C.reset}` : ''}`);
}

function printThreshold(result) {
  console.log(`  ${C.dim}threshold: ${result.threshold} | amount: $${result.amount.toLocaleString()}${C.reset}`);
}

function printResult(result) {
  if (result.ok) {
    const sigs = Object.values(result.commit.head.signatures);
    console.log(`  ${C.bgGreen}${C.bold} ✓ COMMITTED ${C.reset} commit #${result.commit.sequence} | sigs: ${sigs.length} | sig: ${sigs[0].slice(0, 16)}…`);
  } else {
    console.log(`  ${C.bgRed}${C.bold} ✗ REJECTED ${C.reset} ${result.error}`);
  }
  console.log();
}

// ─── Main Demo ────────────────────────────────────────────

async function main() {
  printHeader('MODALITY TUTORIAL: MULTISIG TRANSACTION AUTHORIZATION');
  console.log(`  ${C.dim}Threshold-based authorization with real ed25519 signatures.`);
  console.log(`  Escalating approval requirements based on transaction size.${C.reset}\n`);

  const treasury = generateIdentity('Treasury');
  const cfo      = generateIdentity('CFO');
  const boardA   = generateIdentity('Board Member A');
  const boardB   = generateIdentity('Board Member B');
  const mallory  = generateIdentity('Mallory');

  console.log(`  ${C.bold}Agent Identities:${C.reset}`);
  for (const a of [treasury, cfo, boardA, boardB, mallory]) {
    const c = agentColor[a.name] || C.white;
    console.log(`  ${c}${C.bold}${a.name}${C.reset} ${C.dim}pubkey: ${a.shortKey}${C.reset}`);
  }
  console.log();

  console.log(`  ${C.bold}Authorization Thresholds:${C.reset}`);
  console.log(`  ${C.yellow}rule:${C.reset} ${C.dim}< $10K   → treasury alone (1-of-1)${C.reset}`);
  console.log(`  ${C.yellow}rule:${C.reset} ${C.dim}$10K–$100K → treasury + 1 co-signer (2-of-3)${C.reset}`);
  console.log(`  ${C.yellow}rule:${C.reset} ${C.dim}> $100K  → treasury + CFO + 1 board member (3-of-4)${C.reset}`);
  console.log();

  const contract = new MultisigContract({ treasury, cfo, boardA, boardB });

  // ─── Scenario 1: Small payment ───
  printHeader('SCENARIO 1: Small Payment ($5K) — Treasury Alone');
  printAgent(treasury, 'APPROVE', '{ to: "Vendor A", amount: $5,000 }');
  let result = contract.submit('Payment to Vendor A', 5000, [treasury], { to: 'Vendor A' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 2: Medium payment, treasury alone ───
  printHeader('SCENARIO 2: Medium Payment ($50K) — Treasury Alone');
  printAgent(treasury, 'APPROVE', '{ to: "Contractor B", amount: $50,000 }');
  result = contract.submit('Payment to Contractor B', 50000, [treasury], { to: 'Contractor B' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 3: Medium payment, properly co-signed ───
  printHeader('SCENARIO 3: Medium Payment ($50K) — Treasury + Board Member A');
  printAgent(treasury, 'APPROVE', '{ to: "Contractor B", amount: $50,000 }');
  printAgent(boardA, 'CO-SIGN', '(board member authorizes)');
  result = contract.submit('Payment to Contractor B', 50000, [treasury, boardA], { to: 'Contractor B' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 4: Large payment, insufficient signers ───
  printHeader('SCENARIO 4: Large Payment ($250K) — Treasury + 1 Co-signer Only');
  printAgent(treasury, 'APPROVE', '{ to: "Acquisition Corp", amount: $250,000 }');
  printAgent(boardA, 'CO-SIGN', '(board member)');
  result = contract.submit('Acquisition payment', 250000, [treasury, boardA], { to: 'Acquisition Corp' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 5: Large payment, full quorum ───
  printHeader('SCENARIO 5: Large Payment ($250K) — Full Quorum');
  printAgent(treasury, 'APPROVE', '{ to: "Acquisition Corp", amount: $250,000 }');
  printAgent(cfo, 'CO-SIGN', '(CFO authorizes)');
  printAgent(boardB, 'CO-SIGN', '(board member authorizes)');
  result = contract.submit('Acquisition payment', 250000, [treasury, cfo, boardB], { to: 'Acquisition Corp' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 6: Mallory tries alone ───
  printHeader('SCENARIO 6: Attacker (Mallory) Tries to Approve Transaction');
  printAgent(mallory, 'APPROVE', '{ to: "Mallory Wallet", amount: $1,000 }');
  result = contract.submit('Payment to Mallory Wallet', 1000, [mallory], { to: 'Mallory Wallet' });
  printThreshold(result);
  printResult(result);

  // ─── Scenario 7: Mallory + Treasury ───
  printHeader('SCENARIO 7: Mallory + Treasury Try Medium Payment');
  printAgent(treasury, 'APPROVE', '{ to: "Shell Corp", amount: $50,000 }');
  printAgent(mallory, 'CO-SIGN', '(attacker posing as board member)');
  result = contract.submit('Payment to Shell Corp', 50000, [treasury, mallory], { to: 'Shell Corp' });
  printThreshold(result);
  printResult(result);

  // ─── Audit Trail ───
  printHeader('AUDIT TRAIL');
  const audit = contract.audit();
  console.log(`  ${audit.valid ? C.green + '✓' : C.red + '✗'} ${C.bold}${audit.commitCount} commits — all signatures verified${C.reset}\n`);

  const allAgents = [treasury, cfo, boardA, boardB, mallory];
  for (const commit of contract.commits) {
    const signerKeys = Object.keys(commit.head.signatures);
    const signerNames = signerKeys.map(k => allAgents.find(a => a.publicKey === k)?.name || 'unknown');
    const amt = commit.body.amount;
    console.log(`  ${C.dim}#${commit.sequence}${C.reset} ${C.bold}${commit.body.description}${C.reset} ${C.dim}($${amt.toLocaleString()})${C.reset}`);
    console.log(`     ${C.dim}signers: ${signerNames.join(', ')}${C.reset}`);
  }

  // ─── Summary ───
  printHeader('DEMO COMPLETE');
  console.log(`  ${C.bold}What was demonstrated:${C.reset}`);
  console.log(`  ${C.green}✓${C.reset} Threshold-based authorization — escalating approval requirements`);
  console.log(`  ${C.green}✓${C.reset} Single-signer for small transactions`);
  console.log(`  ${C.green}✓${C.reset} Multi-sig enforcement for medium and large transactions`);
  console.log(`  ${C.green}✓${C.reset} Unknown signer rejection — unauthorized agents blocked`);
  console.log(`  ${C.green}✓${C.reset} Invalid co-signer detection — attacker can't substitute for board`);
  console.log(`  ${C.green}✓${C.reset} Real ed25519 signatures on every commit`);
  console.log(`  ${C.green}✓${C.reset} Full audit trail with cryptographic verification`);
  console.log();
  console.log(`  ${C.dim}This is Modality. Verifiable contracts for the agentic economy.${C.reset}`);
  console.log(`  ${C.dim}https://modality.org${C.reset}\n`);
}

main().catch(console.error);
