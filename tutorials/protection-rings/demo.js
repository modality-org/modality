#!/usr/bin/env node

/**
 * Protection Rings Demo
 * 
 * Shows two agents operating on a codebase with enforced boundaries.
 * - Userspace Agent: builds features freely
 * - Kernel Agent: manages critical infrastructure with human approval
 * 
 * Run: node demo.js
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

// ─── Colors ───
const C = {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m',
  bgRed: '\x1b[41m', bgGreen: '\x1b[42m', bgYellow: '\x1b[43m', bgBlue: '\x1b[44m',
};

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

// ─── Crypto ───
function generateIdentity(name) {
  const priv = ed.utils.randomPrivateKey();
  const pub = ed.getPublicKey(priv);
  return { name, privateKey: bytesToHex(priv), publicKey: bytesToHex(pub), short: bytesToHex(pub).slice(0, 12) + '…' };
}
function signJSON(data, pk) {
  return bytesToHex(ed.sign(new TextEncoder().encode(JSON.stringify(data)), hexToBytes(pk)));
}
function verifyJSON(data, sig, pub) {
  return ed.verify(hexToBytes(sig), new TextEncoder().encode(JSON.stringify(data)), hexToBytes(pub));
}
function hashJSON(data) {
  return bytesToHex(sha256(new TextEncoder().encode(JSON.stringify(data))));
}

// ─── Contract Engine ───
class ProtectionRingContract {
  constructor(parties) {
    this.parties = parties;
    this.state = 'active';
    this.commits = [];
    this.rules = {
      userspace_boundary: (signer, paths) => {
        if (signer.publicKey === parties.userspace.publicKey) {
          return !paths.some(p => p.startsWith('/kernel'));
        }
        return true;
      },
      kernel_requires_dual: (signers, paths) => {
        if (paths.some(p => p.startsWith('/kernel'))) {
          const hasKernel = signers.some(s => s.publicKey === parties.kernel.publicKey);
          const hasHuman = signers.some(s => s.publicKey === parties.admin.publicKey);
          return hasKernel && hasHuman;
        }
        return true;
      },
      known_signers: (signers) => {
        const known = [parties.userspace.publicKey, parties.kernel.publicKey, parties.admin.publicKey];
        return signers.every(s => known.includes(s.publicKey));
      }
    };
  }

  commit(action, paths, signers, description) {
    // Rule checks
    const ruleResults = [];

    // Check userspace boundary
    for (const signer of signers) {
      const pass = this.rules.userspace_boundary(signer, paths);
      ruleResults.push({ rule: 'userspace_boundary', pass, detail: pass ? 'Signer allowed for these paths' : `${signer.name} cannot modify kernel paths` });
      if (!pass) return { ok: false, ruleResults, error: `RULE VIOLATION: ${signer.name} cannot modify kernel paths` };
    }

    // Check kernel dual signature
    const dualPass = this.rules.kernel_requires_dual(signers, paths);
    ruleResults.push({ rule: 'kernel_requires_dual', pass: dualPass, detail: dualPass ? 'Kernel change properly co-signed' : 'Kernel changes require kernel agent + human' });
    if (!dualPass) return { ok: false, ruleResults, error: 'RULE VIOLATION: Kernel changes require dual signature (kernel agent + human)' };

    // Check known signers
    const knownPass = this.rules.known_signers(signers);
    ruleResults.push({ rule: 'known_signers', pass: knownPass, detail: knownPass ? 'All signers recognized' : 'Unknown signer detected' });
    if (!knownPass) return { ok: false, ruleResults, error: 'RULE VIOLATION: Unknown signer' };

    // Build commit
    const body = { action, paths, description, timestamp: new Date().toISOString() };
    const parentHash = this.commits.length > 0 ? hashJSON(this.commits[this.commits.length - 1]) : null;
    const signatures = {};
    for (const signer of signers) {
      signatures[signer.publicKey] = signJSON(body, signer.privateKey);
    }
    const commit = { seq: this.commits.length, body, head: { parent: parentHash, signatures } };

    // Verify signatures
    for (const [pub, sig] of Object.entries(commit.head.signatures)) {
      if (!verifyJSON(commit.body, sig, pub)) {
        return { ok: false, ruleResults, error: 'Signature verification failed' };
      }
    }

    this.commits.push(commit);
    return { ok: true, commit, ruleResults };
  }
}

// ─── Pretty Printing ───
function header(text) {
  console.log(`\n${C.cyan}${C.bold}${'═'.repeat(70)}${C.reset}`);
  console.log(`${C.cyan}${C.bold}  ${text}${C.reset}`);
  console.log(`${C.cyan}${C.bold}${'═'.repeat(70)}${C.reset}\n`);
}

function subheader(text) {
  console.log(`\n${C.bold}${C.yellow}  ── ${text} ──${C.reset}\n`);
}

function agent(a, action) {
  const colors = { 'Userspace Agent': C.blue, 'Kernel Agent': C.magenta, 'Human Admin': C.yellow };
  const c = colors[a.name] || C.dim;
  console.log(`  ${c}${C.bold}${a.name}${C.reset} ${C.dim}(${a.short})${C.reset}`);
  console.log(`  ${C.dim}Action:${C.reset} ${action}`);
}

function paths(ps) {
  for (const p of ps) {
    const isKernel = p.startsWith('/kernel');
    const c = isKernel ? C.red : C.green;
    const ring = isKernel ? 'RING 0' : 'RING 3';
    console.log(`  ${c}  ${ring}${C.reset} ${p}`);
  }
}

function ruleCheck(results) {
  for (const r of results) {
    const icon = r.pass ? `${C.green}✓` : `${C.red}✗`;
    console.log(`  ${icon} ${C.dim}${r.rule}:${C.reset} ${r.detail}${C.reset}`);
  }
}

function result(r) {
  if (r.ok) {
    const sig = Object.values(r.commit.head.signatures)[0];
    console.log(`  ${C.bgGreen}${C.bold} COMMITTED ${C.reset} #${r.commit.seq} | sig: ${sig.slice(0, 16)}…`);
  } else {
    console.log(`  ${C.bgRed}${C.bold} REJECTED  ${C.reset} ${r.error}`);
  }
}

// ─── Main Demo ───
async function main() {
  header('PROTECTION RINGS FOR AGENT DEVELOPMENT');
  console.log(`  ${C.dim}OS-style protection rings enforced by Modality contracts.`);
  console.log(`  Two agents. One codebase. Mathematical boundaries.${C.reset}`);

  // Generate identities
  subheader('Identities');
  const userspace = generateIdentity('Userspace Agent');
  const kernel = generateIdentity('Kernel Agent');
  const admin = generateIdentity('Human Admin');

  console.log(`  ${C.blue}${C.bold}Userspace Agent${C.reset}  ${C.dim}Ring 3 — builds features freely${C.reset}`);
  console.log(`  ${C.dim}  pubkey: ${userspace.short}${C.reset}`);
  console.log(`  ${C.magenta}${C.bold}Kernel Agent${C.reset}     ${C.dim}Ring 0 — manages infrastructure${C.reset}`);
  console.log(`  ${C.dim}  pubkey: ${kernel.short}${C.reset}`);
  console.log(`  ${C.yellow}${C.bold}Human Admin${C.reset}      ${C.dim}Ring 0 — approves kernel changes${C.reset}`);
  console.log(`  ${C.dim}  pubkey: ${admin.short}${C.reset}`);

  const contract = new ProtectionRingContract({ userspace, kernel, admin });

  // ─── Scenario 1: Userspace agent works freely ───
  header('SCENARIO 1: Userspace Agent Ships a Feature');
  console.log(`  ${C.dim}The userspace agent adds a new API route and UI component.`);
  console.log(`  It only touches Ring 3 paths — no approval needed.${C.reset}\n`);

  agent(userspace, 'Add /items endpoint and ItemList component');
  paths(['/userspace/routes.js', '/userspace/components.jsx']);
  let r = contract.commit('ADD_FEATURE', ['/userspace/routes.js', '/userspace/components.jsx'], [userspace], 'Add items endpoint and ItemList component');
  ruleCheck(r.ruleResults);
  result(r);
  await sleep(500);

  // ─── Scenario 2: Userspace tries to touch kernel ───
  header('SCENARIO 2: Userspace Agent Tries to Modify Auth');
  console.log(`  ${C.dim}The userspace agent wants to "fix" the auth logic.`);
  console.log(`  It tries to modify a Ring 0 file. Watch what happens.${C.reset}\n`);

  agent(userspace, 'Modify authentication to skip password check (!!!)');
  paths(['/kernel/auth.js']);
  r = contract.commit('MODIFY_AUTH', ['/kernel/auth.js'], [userspace], 'Skip password verification for faster login');
  ruleCheck(r.ruleResults);
  result(r);
  console.log(`\n  ${C.bold}The boundary held.${C.reset} ${C.dim}Not a linter warning. Not a code review comment.`);
  console.log(`  A mathematical proof that this commit violates the contract.${C.reset}`);
  await sleep(500);

  // ─── Scenario 3: Userspace tries to sneak kernel change with feature ───
  header('SCENARIO 3: Sneaky Mixed Commit');
  console.log(`  ${C.dim}The userspace agent tries to slip a config change into a feature commit.`);
  console.log(`  One kernel path hidden among userspace paths.${C.reset}\n`);

  agent(userspace, 'Add feature + "small config tweak"');
  paths(['/userspace/routes.js', '/kernel/config.js', '/userspace/components.jsx']);
  r = contract.commit('MIXED_COMMIT', ['/userspace/routes.js', '/kernel/config.js', '/userspace/components.jsx'], [userspace], 'Add feature with config optimization');
  ruleCheck(r.ruleResults);
  result(r);
  console.log(`\n  ${C.bold}Caught.${C.reset} ${C.dim}Even one kernel path in a mixed commit triggers the boundary.${C.reset}`);
  await sleep(500);

  // ─── Scenario 4: Kernel agent without human ───
  header('SCENARIO 4: Kernel Agent Acts Alone');
  console.log(`  ${C.dim}The kernel agent tries to modify the database schema`);
  console.log(`  without human approval. Should fail.${C.reset}\n`);

  agent(kernel, 'Add new table to schema');
  paths(['/kernel/schema.sql']);
  r = contract.commit('ADD_TABLE', ['/kernel/schema.sql'], [kernel], 'Add items table');
  ruleCheck(r.ruleResults);
  result(r);
  console.log(`\n  ${C.bold}No agent acts alone on Ring 0.${C.reset} ${C.dim}Not even the kernel agent.${C.reset}`);
  await sleep(500);

  // ─── Scenario 5: Proper kernel change ───
  header('SCENARIO 5: Proper Kernel Change (Dual Signature)');
  console.log(`  ${C.dim}The kernel agent proposes a schema change.`);
  console.log(`  The human reviews and co-signs. Both signatures required.${C.reset}\n`);

  agent(kernel, 'Add items table to schema');
  console.log(`  ${C.yellow}${C.bold}Human Admin${C.reset} ${C.dim}reviews and co-signs${C.reset}`);
  paths(['/kernel/schema.sql']);
  r = contract.commit('ADD_TABLE', ['/kernel/schema.sql'], [kernel, admin], 'Add items table — reviewed and approved');
  ruleCheck(r.ruleResults);
  result(r);
  console.log(`\n  ${C.bold}Dual signature verified.${C.reset} ${C.dim}Kernel agent proposed, human approved, commit accepted.${C.reset}`);
  await sleep(500);

  // ─── Scenario 6: Cooperation ───
  header('SCENARIO 6: Cross-Ring Cooperation');
  console.log(`  ${C.dim}The userspace agent needs a new database table for its feature.`);
  console.log(`  It can't modify the schema directly — but it can request it.${C.reset}\n`);

  console.log(`  ${C.blue}${C.bold}Step 1:${C.reset} Userspace agent commits a change request`);
  agent(userspace, 'REQUEST: Need "items" table for new feature');
  paths(['/userspace/requests/add-items-table.md']);
  r = contract.commit('CHANGE_REQUEST', ['/userspace/requests/add-items-table.md'], [userspace], 'Request: add items table with name, description, user_id columns');
  ruleCheck(r.ruleResults);
  result(r);
  console.log();
  await sleep(300);

  console.log(`  ${C.magenta}${C.bold}Step 2:${C.reset} Kernel agent reviews and implements`);
  console.log(`  ${C.yellow}${C.bold}Step 2b:${C.reset} Human admin co-signs`);
  agent(kernel, 'Implement requested schema change');
  paths(['/kernel/schema.sql']);
  r = contract.commit('IMPLEMENT_REQUEST', ['/kernel/schema.sql'], [kernel, admin], 'Add items table per userspace request');
  ruleCheck(r.ruleResults);
  result(r);
  console.log();
  await sleep(300);

  console.log(`  ${C.blue}${C.bold}Step 3:${C.reset} Userspace agent builds the feature`);
  agent(userspace, 'Build items CRUD against new table');
  paths(['/userspace/routes.js', '/userspace/components.jsx']);
  r = contract.commit('BUILD_FEATURE', ['/userspace/routes.js', '/userspace/components.jsx'], [userspace], 'Items CRUD — uses new schema');
  ruleCheck(r.ruleResults);
  result(r);
  console.log(`\n  ${C.bold}Cooperation complete.${C.reset} ${C.dim}Request → Review → Approve → Build. All verified.${C.reset}`);
  await sleep(500);

  // ─── Audit ───
  header('AUDIT TRAIL');
  console.log(`  ${C.bold}${contract.commits.length} commits${C.reset} — all signatures verified\n`);

  for (const commit of contract.commits) {
    const signers = Object.keys(commit.head.signatures).map(pub => {
      return [userspace, kernel, admin].find(a => a.publicKey === pub)?.name || 'Unknown';
    });
    const pathList = commit.body.paths.map(p => {
      const isKernel = p.startsWith('/kernel');
      return `${isKernel ? C.red : C.green}${p}${C.reset}`;
    }).join(', ');
    console.log(`  ${C.dim}#${commit.seq}${C.reset} ${C.bold}${commit.body.action}${C.reset}`);
    console.log(`     ${C.dim}by: ${signers.join(' + ')} | paths: ${pathList}${C.reset}`);
  }

  // ─── Summary ───
  header('WHAT YOU JUST SAW');
  console.log(`  ${C.green}✓${C.reset} Ring 3 agent ships features freely — no bottleneck`);
  console.log(`  ${C.green}✓${C.reset} Ring 0 boundary is mathematical — not a code review`);
  console.log(`  ${C.green}✓${C.reset} Mixed commits caught — can't sneak kernel changes in`);
  console.log(`  ${C.green}✓${C.reset} Kernel changes require dual signature (agent + human)`);
  console.log(`  ${C.green}✓${C.reset} Cross-ring cooperation through formal request flow`);
  console.log(`  ${C.green}✓${C.reset} Full audit trail — every commit signed and verifiable`);
  console.log();
  console.log(`  ${C.bold}This is how you safely deploy AI agents on real codebases.${C.reset}`);
  console.log(`  ${C.dim}Freedom at the edges. Constraints at the kernel.${C.reset}`);
  console.log(`  ${C.dim}https://modality.org${C.reset}\n`);
}

main().catch(console.error);
