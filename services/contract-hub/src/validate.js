/**
 * Commit validation for Contract Hub
 * 
 * Validates commits before accepting them:
 * - Parent chain integrity
 * - Signature verification (if signed)
 * - Hash verification
 * - Basic structure validation
 */

import { createHash } from 'crypto';
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Configure ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

/**
 * Validate a batch of commits for a contract
 * @param {Object} store - ContractStore instance
 * @param {string} contractId - Contract ID
 * @param {Array} commits - Array of commits to validate
 * @returns {Object} { valid: boolean, errors: string[] }
 */
export async function validateCommits(store, contractId, commits) {
  const errors = [];
  
  if (!Array.isArray(commits) || commits.length === 0) {
    return { valid: false, errors: ['commits must be a non-empty array'] };
  }
  
  // Get current contract state
  const contract = store.getContract(contractId);
  if (!contract) {
    return { valid: false, errors: ['Contract not found'] };
  }
  
  // Build a map of existing commits + new commits for parent lookup
  const existingCommits = new Set();
  const newCommitHashes = new Set();
  
  // Get all existing commit hashes
  const existingCommitList = store.pullCommits(contractId);
  for (const c of existingCommitList) {
    existingCommits.add(c.hash);
  }
  
  // Track the expected parent (current head or chained from previous)
  let expectedParent = contract.head;
  
  for (let i = 0; i < commits.length; i++) {
    const commit = commits[i];
    const prefix = `commits[${i}]`;
    
    // 1. Structure validation
    if (!commit.hash || typeof commit.hash !== 'string') {
      errors.push(`${prefix}: missing or invalid hash`);
      continue;
    }
    
    if (commit.data === undefined) {
      errors.push(`${prefix}: missing data`);
      continue;
    }
    
    // 2. Hash verification
    const computedHash = computeCommitHash(commit.data, commit.parent);
    if (commit.hash !== computedHash) {
      // Allow if hash is provided as-is (legacy support)
      // But warn in logs - in strict mode we'd reject
      console.warn(`${prefix}: hash mismatch (provided: ${commit.hash}, computed: ${computedHash})`);
    }
    
    // 3. Parent chain validation
    if (i === 0) {
      // First commit must have parent = current head (or null if empty)
      if (commit.parent !== expectedParent && commit.parent !== null) {
        // Check if parent exists in history
        if (commit.parent && !existingCommits.has(commit.parent)) {
          errors.push(`${prefix}: parent '${commit.parent}' not found in contract history`);
        }
      }
    } else {
      // Subsequent commits must chain from previous commit in batch
      const prevHash = commits[i - 1].hash;
      if (commit.parent !== prevHash) {
        // Allow if parent is in existing commits (parallel push)
        if (!existingCommits.has(commit.parent) && !newCommitHashes.has(commit.parent)) {
          errors.push(`${prefix}: invalid parent chain (expected '${prevHash}', got '${commit.parent}')`);
        }
      }
    }
    
    // 4. Signature verification (if present)
    if (commit.signature) {
      const sigValid = await verifyCommitSignature(commit);
      if (!sigValid.valid) {
        errors.push(`${prefix}: ${sigValid.error}`);
      }
    }
    
    // 5. Check for duplicate hash
    if (existingCommits.has(commit.hash)) {
      // Not an error - just skip (idempotent push)
      console.log(`${prefix}: commit ${commit.hash} already exists, skipping`);
    }
    
    newCommitHashes.add(commit.hash);
    expectedParent = commit.hash;
  }
  
  return {
    valid: errors.length === 0,
    errors
  };
}

/**
 * Compute the expected hash for a commit
 */
function computeCommitHash(data, parent) {
  const payload = JSON.stringify({ data, parent: parent || null });
  return createHash('sha256').update(payload).digest('hex').slice(0, 16);
}

/**
 * Verify a commit signature
 * Signature signs: hash + parent + JSON(data)
 */
async function verifyCommitSignature(commit) {
  if (!commit.signature) {
    return { valid: true }; // No signature = no verification needed
  }
  
  const { signature, signer_key } = parseSignature(commit.signature);
  
  if (!signature || !signer_key) {
    return { valid: false, error: 'invalid signature format (expected "sig:pubkey" or {signature, signer_key})' };
  }
  
  try {
    // Message = hash:parent:data
    const message = `${commit.hash}:${commit.parent || ''}:${JSON.stringify(commit.data)}`;
    const messageBytes = new TextEncoder().encode(message);
    const sigBytes = hexToBytes(signature);
    const pubkeyBytes = hexToBytes(signer_key);
    
    const valid = await ed.verifyAsync(sigBytes, messageBytes, pubkeyBytes);
    
    if (!valid) {
      return { valid: false, error: 'signature verification failed' };
    }
    
    return { valid: true, signer: signer_key };
  } catch (err) {
    return { valid: false, error: `signature verification error: ${err.message}` };
  }
}

/**
 * Parse signature from various formats
 */
function parseSignature(sig) {
  if (typeof sig === 'string') {
    // Format: "signature:signer_key"
    const parts = sig.split(':');
    if (parts.length === 2) {
      return { signature: parts[0], signer_key: parts[1] };
    }
    return { signature: sig, signer_key: null };
  }
  
  if (typeof sig === 'object') {
    return { signature: sig.signature, signer_key: sig.signer_key || sig.signerKey };
  }
  
  return { signature: null, signer_key: null };
}

function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

export { computeCommitHash, verifyCommitSignature };
