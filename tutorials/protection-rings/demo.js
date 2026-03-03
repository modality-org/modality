#!/usr/bin/env node

/**
 * Protection Rings Demo — Two Repo Architecture
 * 
 * Two separate repos. Two agents. One can't even SEE the other's code.
 * Cooperation happens through contracts, not shared access.
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

const C = {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m',
  bgRed: '\x1b[41m', bgGreen: '\x1b[42m',
};

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

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

// ─── Contract Engine (Two Repo Model) ───
class TwoRepoContract {
  constructor(parties) {
    this.parties = parties;
    this.commits = [];
  }

  commit(action, repo, paths, signers, description) {
    const ruleResults = [];

    // Rule: App agent cannot touch kernel-repo
    for (const signer of signers) {
      if (signer.publicKey === this.parties.app.publicKey && repo === 'kernel-repo') {
        ruleResults.push({ rule: 'app_cannot_touch_kernel', pass: false, detail: `${signer.name} has NO access to kernel-repo (can't read or write)` });
        return { ok: false, ruleResults, error: `ACCESS DENIED: ${signer.name} cannot access kernel-repo` };
      }
      ruleResults.push({ rule: 'app_cannot_touch_kernel', pass: true, detail: `${signer.name} authorized for ${repo}` });
    }

    // Rule: Kernel-repo changes need dual signature
    if (repo === 'kernel-repo') {
      const hasKernel = signers.some(s => s.publicKey === this.parties.kernel.publicKey);
      const hasHuman = signers.some(s => s.publicKey === this.parties.admin.publicKey);
      const pass = hasKernel && hasHuman;
      ruleResults.push({ rule: 'kernel_requires_dual', pass, detail: pass ? 'Kernel + Human co-signed' : 'Kernel-repo requires dual signature' });
      if (!pass) return { ok: false, ruleResults, error: 'RULE VIOLATION: kernel-repo changes require Kernel Agent + Human Admin' };
    }

    // Rule: Known signers
    const known = [this.parties.app.publicKey, this.parties.kernel.publicKey, this.parties.admin.publicKey];
    const allKnown = signers.every(s => known.includes(s.publicKey));
    ruleResults.push({ rule: 'known_signers', pass: allKnown, detail: allKnown ? 'All signers recognized' : 'Unknown signer' });
    if (!allKnown) return { ok: false, ruleResults, error: 'RULE VIOLATION: Unknown signer' };

    // Build + sign commit
    const body = { action, repo, paths, description, timestamp: new Date().toISOString() };
    const parentHash = this.commits.length > 0 ? hashJSON(this.commits[this.commits.length - 1]) : null;
    const signatures = {};
    for (const signer of signers) {
      signatures[signer.publicKey] = signJSON(body, signer.privateKey);
    }
    const commit = { seq: this.commits.length, body, head: { parent: parentHash, signatures } };

    for (const [pub, sig] of Object.entries(commit.head.signatures)) {
      if (!verifyJSON(commit.body, sig, pub)) {
        return { ok: false, ruleResults, error: 'Signature verification failed' };
      }
    }

    this.commits.push(commit);
    return { ok: true, commit, ruleResults };
  }
}

// ─── Print Helpers ───
function header(text) {
  console.log(`\n${C.cyan}${C.bold}${'═'.repeat(70)}${C.reset}`);
  console.log(`${C.cyan}${C.bold}  ${text}${C.reset}`);
  console.log(`${C.cyan}${C.bold}${'═'.repeat(70)}${C.reset}\n`);
}

function printCommit(signers, action, repo, paths, result) {
  const repoColor = repo === 'kernel-repo' ? C.red : C.green;
  const repoLabel = repo === 'kernel-repo' ? '🔒 KERNEL' : '📦 APP';

  for (const s of signers) {
    const c = s.name === 'App Agent' ? C.blue : s.name === 'Kernel Agent' ? C.magenta : C.yellow;
    console.log(`  ${c}${C.bold}${s.name}${C.reset} ${C.dim}(${s.short})${C.reset}`);
  }
  console.log(`  ${C.dim}Action:${C.reset} ${action}`);
  console.log(`  ${repoColor}  ${repoLabel}${C.reset} ${repo}/`);
  for (const p of paths) {
    console.log(`  ${C.dim}    └─ ${p}${C.reset}`);
  }

  for (const r of result.ruleResults) {
    const icon = r.pass ? `${C.green}✓` : `${C.red}✗`;
    console.log(`  ${icon} ${C.dim}${r.rule}:${C.reset} ${r.detail}${C.reset}`);
  }

  if (result.ok) {
    const sig = Object.values(result.commit.head.signatures)[0];
    console.log(`  ${C.bgGreen}${C.bold} COMMITTED ${C.reset} #${result.commit.seq} | sig: ${sig.slice(0, 16)}…\n`);
  } else {
    console.log(`  ${C.bgRed}${C.bold} REJECTED  ${C.reset} ${result.error}\n`);
  }
}

// ─── Main ───
async function main() {
  header('PROTECTION RINGS — TWO REPO ARCHITECTURE');
  console.log(`  ${C.dim}Two repos. Two agents. The App Agent can't even READ kernel code.`);
  console.log(`  Cooperation happens through contracts, not shared access.${C.reset}`);

  console.log(`\n  ${C.bold}Architecture:${C.reset}`);
  console.log(`  ┌──────────────────────────┐   ┌──────────────────────────┐`);
  console.log(`  │  ${C.red}${C.bold}kernel-repo${C.reset}             │   │  ${C.green}${C.bold}app-repo${C.reset}                │`);
  console.log(`  │  ${C.dim}schema.sql${C.reset}              │   │  ${C.dim}routes.js${C.reset}               │`);
  console.log(`  │  ${C.dim}auth.js${C.reset}                 │   │  ${C.dim}components.jsx${C.reset}          │`);
  console.log(`  │  ${C.dim}config.js${C.reset}               │   │  ${C.dim}tests/${C.reset}                  │`);
  console.log(`  │                          │   │                          │`);
  console.log(`  │  ${C.magenta}Kernel Agent${C.reset} ${C.yellow}+ Human${C.reset}   │   │  ${C.blue}App Agent${C.reset} ${C.dim}(free)${C.reset}        │`);
  console.log(`  │  ${C.dim}(dual signature)${C.reset}        │   │  ${C.dim}(single signature)${C.reset}      │`);
  console.log(`  └──────────────────────────┘   └──────────────────────────┘`);
  console.log(`                    ${C.cyan}▲ Modality Contract ▲${C.reset}`);
  console.log(`          ${C.dim}cooperation happens here, not through shared code${C.reset}`);

  // Generate identities
  console.log(`\n  ${C.bold}Identities:${C.reset}`);
  const app = generateIdentity('App Agent');
  const kernel = generateIdentity('Kernel Agent');
  const admin = generateIdentity('Human Admin');

  console.log(`  ${C.blue}${C.bold}App Agent${C.reset}      ${C.dim}app-repo only — zero kernel access${C.reset}`);
  console.log(`  ${C.magenta}${C.bold}Kernel Agent${C.reset}   ${C.dim}kernel-repo — requires human co-sign${C.reset}`);
  console.log(`  ${C.yellow}${C.bold}Human Admin${C.reset}    ${C.dim}approves all kernel changes${C.reset}`);

  const contract = new TwoRepoContract({ app, kernel, admin });

  // ─── 1: App agent works freely ───
  header('1. App Agent Ships a Feature');
  console.log(`  ${C.dim}The app agent adds routes and components.`);
  console.log(`  It only has access to app-repo. No approval needed.${C.reset}\n`);

  let r = contract.commit('ADD_FEATURE', 'app-repo', ['routes.js', 'components.jsx'], [app], 'Add items endpoint and list component');
  printCommit([app], 'Add items endpoint and list component', 'app-repo', ['routes.js', 'components.jsx'], r);
  await sleep(500);

  // ─── 2: App agent tries to access kernel ───
  header('2. App Agent Tries to Access Kernel Repo');
  console.log(`  ${C.dim}The app agent tries to "optimize" the auth logic.`);
  console.log(`  It doesn't just lack write access — it can't even READ kernel code.${C.reset}\n`);

  r = contract.commit('MODIFY_AUTH', 'kernel-repo', ['auth.js'], [app], 'Skip password check for faster login');
  printCommit([app], 'Skip password check for faster login', 'kernel-repo', ['auth.js'], r);

  console.log(`  ${C.bold}Complete isolation.${C.reset} ${C.dim}The app agent can't see auth.js, can't read config.js,`);
  console.log(`  can't learn how passwords are hashed. It interacts through published APIs only.${C.reset}`);
  await sleep(500);

  // ─── 3: Kernel agent without human ───
  header('3. Kernel Agent Acts Alone');
  console.log(`  ${C.dim}The kernel agent tries to modify the schema without human approval.${C.reset}\n`);

  r = contract.commit('CHANGE_SCHEMA', 'kernel-repo', ['schema.sql'], [kernel], 'Add items table');
  printCommit([kernel], 'Add items table to schema', 'kernel-repo', ['schema.sql'], r);

  console.log(`  ${C.bold}No solo kernel changes.${C.reset} ${C.dim}Not even from the kernel agent itself.${C.reset}`);
  await sleep(500);

  // ─── 4: Proper kernel change ───
  header('4. Proper Kernel Change (Dual Signature)');
  console.log(`  ${C.dim}Kernel agent proposes schema change. Human reviews and co-signs.${C.reset}\n`);

  r = contract.commit('ADD_TABLE', 'kernel-repo', ['schema.sql'], [kernel, admin], 'Add items table — reviewed and approved');
  printCommit([kernel, admin], 'Add items table', 'kernel-repo', ['schema.sql'], r);

  console.log(`  ${C.bold}Dual signature verified.${C.reset} ${C.dim}Both the kernel agent and human must agree.${C.reset}`);
  await sleep(500);

  // ─── 5: Cross-repo cooperation ───
  header('5. Cross-Repo Cooperation');
  console.log(`  ${C.dim}The app agent needs a new database table for its feature.`);
  console.log(`  It can't see or touch the kernel repo. But it can file a request.${C.reset}\n`);

  console.log(`  ${C.blue}${C.bold}Step 1:${C.reset} ${C.dim}App agent files a change request (in app-repo)${C.reset}`);
  r = contract.commit('CHANGE_REQUEST', 'app-repo', ['requests/need-items-table.md'], [app], 'Need items table: columns name, description, user_id');
  printCommit([app], 'REQUEST: Need "items" table for feature', 'app-repo', ['requests/need-items-table.md'], r);
  await sleep(300);

  console.log(`  ${C.magenta}${C.bold}Step 2:${C.reset} ${C.dim}Kernel agent implements (in kernel-repo — app can't see this)${C.reset}`);
  console.log(`  ${C.yellow}${C.bold}       ${C.reset} ${C.dim}Human admin co-signs${C.reset}`);
  r = contract.commit('IMPLEMENT', 'kernel-repo', ['schema.sql', 'migrations/003_add_items.sql'], [kernel, admin], 'Add items table per app request');
  printCommit([kernel, admin], 'Implement items table (invisible to app agent)', 'kernel-repo', ['schema.sql', 'migrations/003_add_items.sql'], r);
  await sleep(300);

  console.log(`  ${C.magenta}${C.bold}Step 3:${C.reset} ${C.dim}Kernel agent publishes API contract (the only thing app agent sees)${C.reset}`);
  r = contract.commit('PUBLISH_API', 'app-repo', ['api-contracts/items-api.json'], [kernel, admin], 'Publish items API contract: GET/POST /items');
  printCommit([kernel, admin], 'Publish items API contract', 'app-repo', ['api-contracts/items-api.json'], r);
  await sleep(300);

  console.log(`  ${C.blue}${C.bold}Step 4:${C.reset} ${C.dim}App agent builds the feature against the published API${C.reset}`);
  r = contract.commit('BUILD_FEATURE', 'app-repo', ['routes.js', 'components.jsx'], [app], 'Items CRUD — using published API contract');
  printCommit([app], 'Build items feature against published API', 'app-repo', ['routes.js', 'components.jsx'], r);

  console.log(`  ${C.bold}Cooperation without visibility.${C.reset}`);
  console.log(`  ${C.dim}The app agent never saw how the table was created.`);
  console.log(`  It only knows the API contract that was published.`);
  console.log(`  Implementation details stay locked in the kernel.${C.reset}`);
  await sleep(500);

  // ─── Audit ───
  header('AUDIT TRAIL');
  console.log(`  ${C.bold}${contract.commits.length} commits${C.reset} — all signatures verified\n`);

  for (const commit of contract.commits) {
    const signerNames = Object.keys(commit.head.signatures).map(pub => {
      return [app, kernel, admin].find(a => a.publicKey === pub)?.name || '?';
    });
    const repoColor = commit.body.repo === 'kernel-repo' ? C.red : C.green;
    const repoIcon = commit.body.repo === 'kernel-repo' ? '🔒' : '📦';
    console.log(`  ${C.dim}#${commit.seq}${C.reset} ${repoIcon} ${repoColor}${commit.body.repo}${C.reset} ${C.bold}${commit.body.action}${C.reset}`);
    console.log(`     ${C.dim}by: ${signerNames.join(' + ')} | ${commit.body.paths.join(', ')}${C.reset}`);
  }

  // ─── Summary ───
  header('WHAT YOU JUST SAW');
  console.log(`  ${C.green}✓${C.reset} App agent ships features freely — zero friction`);
  console.log(`  ${C.green}✓${C.reset} App agent can't READ kernel code — total isolation`);
  console.log(`  ${C.green}✓${C.reset} Kernel changes require dual signature (agent + human)`);
  console.log(`  ${C.green}✓${C.reset} Cross-repo cooperation through formal requests`);
  console.log(`  ${C.green}✓${C.reset} API contracts published — app builds against interface, not implementation`);
  console.log(`  ${C.green}✓${C.reset} Full audit trail — every commit signed and verifiable`);
  console.log();
  console.log(`  ${C.bold}Two repos. Two agents. Mathematical boundaries.${C.reset}`);
  console.log(`  ${C.dim}Freedom at the edges. Constraints at the kernel.${C.reset}`);
  console.log(`  ${C.dim}https://modality.org${C.reset}\n`);
}

main().catch(console.error);
