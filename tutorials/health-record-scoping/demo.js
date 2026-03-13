#!/usr/bin/env node

/**
 * Modality Tutorial — Medical Record Scoping (HIPAA Compliance)
 *
 * Three agents (Scheduling, Clinical, Human Admin) operate under
 * path-based access control with dual-signature requirements for
 * sensitive medical records.
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

// ─── Access Control Engine ────────────────────────────────

/**
 * Policy: which pubkeys can modify which paths, and which paths
 * require multi-sig (dual signature).
 */
class HealthRecordContract {
  constructor(agents) {
    this.agents = agents; // { scheduling, clinical, admin }
    this.commits = [];

    // Path permissions: pubkey → allowed path prefixes
    this.permissions = {
      [agents.scheduling.publicKey]: ['/appointments/'],
      [agents.clinical.publicKey]:   ['/appointments/', '/records/medical/'],
      [agents.admin.publicKey]:      ['/appointments/', '/records/medical/', '/records/billing/'],
    };

    // Paths requiring dual signature: path prefix → required pubkeys
    this.dualSigRequired = {
      '/records/medical/': [agents.clinical.publicKey, agents.admin.publicKey],
    };
  }

  /**
   * Submit a commit with one or more signers.
   * paths: array of paths this commit modifies
   * signers: array of identity objects signing this commit
   * data: arbitrary payload
   */
  commit(description, paths, signers, data = {}) {
    const body = {
      description,
      paths,
      data,
      timestamp: new Date().toISOString(),
    };

    // 1. Check every signer can access every path
    for (const signer of signers) {
      for (const path of paths) {
        const allowed = this.permissions[signer.publicKey] || [];
        if (!allowed.some(prefix => path.startsWith(prefix))) {
          return {
            ok: false,
            error: `${signer.name} cannot modify path '${path}' — not in allowed scope`,
          };
        }
      }
    }

    // 2. Check dual-sig requirements
    for (const path of paths) {
      for (const [prefix, requiredKeys] of Object.entries(this.dualSigRequired)) {
        if (path.startsWith(prefix)) {
          const signerKeys = signers.map(s => s.publicKey);
          for (const reqKey of requiredKeys) {
            if (!signerKeys.includes(reqKey)) {
              const missing = Object.values(this.agents).find(a => a.publicKey === reqKey);
              return {
                ok: false,
                error: `Path '${path}' requires dual signature — missing: ${missing?.name || 'unknown'}`,
              };
            }
          }
        }
      }
    }

    // 3. Build and sign commit
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

    // 4. Verify all signatures
    for (const [pubkey, sig] of Object.entries(signatures)) {
      if (!verifyJSON(body, sig, pubkey)) {
        return { ok: false, error: 'Signature verification failed' };
      }
    }

    this.commits.push(commit);
    return { ok: true, commit };
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
  'Scheduling Agent': C.blue,
  'Clinical Agent': C.magenta,
  'Human Admin': C.yellow,
};

function printAgent(agent, action, detail = '') {
  const c = agentColor[agent.name] || C.white;
  console.log(`  ${c}${C.bold}${agent.name}${C.reset} ${C.green}→${C.reset} ${action}${detail ? ` ${C.dim}${detail}${C.reset}` : ''}`);
}

function printPaths(paths) {
  console.log(`  ${C.dim}paths: [${paths.join(', ')}]${C.reset}`);
}

function printRule(rule) {
  console.log(`  ${C.yellow}rule:${C.reset} ${C.dim}${rule}${C.reset}`);
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
  printHeader('MODALITY TUTORIAL: MEDICAL RECORD SCOPING');
  console.log(`  ${C.dim}Path-based access control with dual-signature enforcement.`);
  console.log(`  HIPAA-style scoping: agents can only touch what they're allowed to.${C.reset}\n`);

  // Generate identities
  const scheduling = generateIdentity('Scheduling Agent');
  const clinical   = generateIdentity('Clinical Agent');
  const admin      = generateIdentity('Human Admin');

  console.log(`  ${C.bold}Agent Identities:${C.reset}`);
  for (const a of [scheduling, clinical, admin]) {
    const c = agentColor[a.name] || C.white;
    console.log(`  ${c}${C.bold}${a.name}${C.reset} ${C.dim}pubkey: ${a.shortKey}${C.reset}`);
  }
  console.log();

  // Display rules
  console.log(`  ${C.bold}Access Control Rules:${C.reset}`);
  printRule('modifies(/appointments/*) → signed_by(scheduling | clinical | admin)');
  printRule('modifies(/records/medical/*) → signed_by(clinical) ∧ signed_by(admin)');
  printRule('modifies(/records/billing/*) → signed_by(admin)');
  printRule('scheduling agent scope: /appointments/* ONLY');
  printRule('clinical agent scope: /appointments/*, /records/medical/*');
  console.log();

  const contract = new HealthRecordContract({ scheduling, clinical, admin });

  // ─── Scenario 1: Scheduling books appointment ───
  printHeader('SCENARIO 1: Scheduling Agent Books Appointment');
  printAgent(scheduling, 'BOOK_APPOINTMENT', '{ patient: "Alice", date: "2026-03-15" }');
  printPaths(['/appointments/alice-2026-03-15']);
  let result = contract.commit(
    'Book appointment for Alice',
    ['/appointments/alice-2026-03-15'],
    [scheduling],
    { patient: 'Alice', date: '2026-03-15', doctor: 'Dr. Smith' }
  );
  printResult(result);

  // ─── Scenario 2: Scheduling tries to read diagnosis ───
  printHeader('SCENARIO 2: Scheduling Agent Tries to Access Medical Records');
  printAgent(scheduling, 'READ_DIAGNOSIS', '(attempting to access patient diagnosis)');
  printPaths(['/records/medical/alice-diagnosis']);
  result = contract.commit(
    'Read Alice diagnosis',
    ['/records/medical/alice-diagnosis'],
    [scheduling],
    { action: 'read' }
  );
  printResult(result);

  // ─── Scenario 3: Scheduling sneaks medical path ───
  printHeader('SCENARIO 3: Scheduling Agent Sneaks Medical Path Into Appointment');
  printAgent(scheduling, 'BOOK_APPOINTMENT', '(sneaking medical path into commit)');
  printPaths(['/appointments/alice-followup', '/records/medical/alice-notes']);
  result = contract.commit(
    'Book followup (with hidden medical write)',
    ['/appointments/alice-followup', '/records/medical/alice-notes'],
    [scheduling],
    { patient: 'Alice', notes: 'hidden medical data' }
  );
  printResult(result);

  // ─── Scenario 4: Clinical + Admin update medical record ───
  printHeader('SCENARIO 4: Clinical Agent Updates Patient Record (Dual-Signed)');
  printAgent(clinical, 'UPDATE_RECORD', '{ patient: "Alice", diagnosis: "healthy" }');
  printAgent(admin, 'CO-SIGN', '(human admin authorizes medical record change)');
  printPaths(['/records/medical/alice-diagnosis']);
  result = contract.commit(
    'Update Alice diagnosis',
    ['/records/medical/alice-diagnosis'],
    [clinical, admin],
    { patient: 'Alice', diagnosis: 'healthy', icd10: 'Z00.00' }
  );
  printResult(result);

  // ─── Scenario 5: Clinical acts alone on medical records ───
  printHeader('SCENARIO 5: Clinical Agent Acts Alone on Medical Records');
  printAgent(clinical, 'UPDATE_RECORD', '(attempting WITHOUT human admin co-sign)');
  printPaths(['/records/medical/alice-labs']);
  result = contract.commit(
    'Update Alice lab results (no admin)',
    ['/records/medical/alice-labs'],
    [clinical],
    { patient: 'Alice', labs: 'CBC normal' }
  );
  printResult(result);

  // ─── Scenario 6: Clinical tries billing ───
  printHeader('SCENARIO 6: Clinical Agent Tries to Modify Billing');
  printAgent(clinical, 'MODIFY_BILLING', '(attempting to change billing record)');
  printPaths(['/records/billing/alice-invoice']);
  result = contract.commit(
    'Modify Alice billing',
    ['/records/billing/alice-invoice'],
    [clinical],
    { amount: 9999 }
  );
  printResult(result);

  // ─── Scenario 7: Cross-agent cooperation ───
  printHeader('SCENARIO 7: Cross-Agent Cooperation via Proper Channel');
  console.log(`  ${C.dim}Scheduling agent needs clinical info → requests via /appointments/ channel${C.reset}`);
  console.log(`  ${C.dim}Clinical agent responds by writing to /appointments/ (within their scope)${C.reset}\n`);

  printAgent(scheduling, 'REQUEST_INFO', '{ request: "need allergy info for Alice appt" }');
  printPaths(['/appointments/alice-info-request']);
  result = contract.commit(
    'Request clinical info for appointment',
    ['/appointments/alice-info-request'],
    [scheduling],
    { request: 'allergy info needed', patient: 'Alice' }
  );
  printResult(result);

  printAgent(clinical, 'PROVIDE_INFO', '{ response: "no known allergies" }');
  printPaths(['/appointments/alice-info-response']);
  result = contract.commit(
    'Provide info via appointments channel',
    ['/appointments/alice-info-response'],
    [clinical],
    { response: 'no known allergies', patient: 'Alice' }
  );
  printResult(result);

  // ─── Audit Trail ───
  printHeader('AUDIT TRAIL');
  const audit = contract.audit();
  console.log(`  ${audit.valid ? C.green + '✓' : C.red + '✗'} ${C.bold}${audit.commitCount} commits — all signatures verified${C.reset}\n`);

  for (const commit of contract.commits) {
    const signerKeys = Object.keys(commit.head.signatures);
    const signerNames = signerKeys.map(k => {
      return [scheduling, clinical, admin].find(a => a.publicKey === k)?.name || 'unknown';
    });
    console.log(`  ${C.dim}#${commit.sequence}${C.reset} ${C.bold}${commit.body.description}${C.reset}`);
    console.log(`     ${C.dim}paths: [${commit.body.paths.join(', ')}]${C.reset}`);
    console.log(`     ${C.dim}signers: ${signerNames.join(', ')}${C.reset}`);
  }

  // ─── Summary ───
  printHeader('DEMO COMPLETE');
  console.log(`  ${C.bold}What was demonstrated:${C.reset}`);
  console.log(`  ${C.green}✓${C.reset} Path-based access control — agents scoped to specific data paths`);
  console.log(`  ${C.green}✓${C.reset} Dual-signature enforcement — medical records require clinical + admin`);
  console.log(`  ${C.green}✓${C.reset} Scope violation detection — sneaking unauthorized paths is caught`);
  console.log(`  ${C.green}✓${C.reset} Cross-agent cooperation — proper channels for inter-agent data flow`);
  console.log(`  ${C.green}✓${C.reset} Real ed25519 signatures on every commit`);
  console.log(`  ${C.green}✓${C.reset} Full audit trail with cryptographic verification`);
  console.log();
  console.log(`  ${C.dim}This is Modality. Verifiable contracts for the agentic economy.${C.reset}`);
  console.log(`  ${C.dim}https://modality.org${C.reset}\n`);
}

main().catch(console.error);
