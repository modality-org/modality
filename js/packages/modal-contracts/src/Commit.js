/**
 * Commit - A single commit in a modal contract log
 */

import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex } from '@noble/hashes/utils';

/**
 * Commit types
 */
export const CommitType = {
  /** Set a value at a path */
  POST: 'POST',
  /** Add a rule */
  RULE: 'RULE',
  /** Perform a domain action */
  ACTION: 'ACTION',
  /** Delete a path */
  DELETE: 'DELETE',
};

/**
 * Represents a commit in a modal contract
 */
export class Commit {
  /**
   * @param {object} options
   * @param {string} options.parent - Parent commit hash (null for genesis)
   * @param {string} options.type - Commit type (POST, RULE, ACTION, DELETE)
   * @param {string} options.path - Path for POST/DELETE
   * @param {any} options.payload - Commit payload
   * @param {number} options.timestamp - Unix timestamp (ms)
   * @param {object[]} options.signatures - Array of {publicKey, signature}
   */
  constructor({ parent, type, path, payload, timestamp, signatures = [] }) {
    this.parent = parent;
    this.type = type;
    this.path = path;
    this.payload = payload;
    this.timestamp = timestamp || Date.now();
    this.signatures = signatures;
    this._hash = null;
  }

  /**
   * Create a POST commit
   * @param {string} parent - Parent hash
   * @param {string} path - Path to set
   * @param {any} value - Value to set
   * @returns {Commit}
   */
  static post(parent, path, value) {
    return new Commit({
      parent,
      type: CommitType.POST,
      path,
      payload: value,
    });
  }

  /**
   * Create a RULE commit
   * @param {string} parent - Parent hash
   * @param {string} ruleContent - Modality rule content
   * @returns {Commit}
   */
  static rule(parent, ruleContent) {
    return new Commit({
      parent,
      type: CommitType.RULE,
      payload: ruleContent,
    });
  }

  /**
   * Create an ACTION commit
   * @param {string} parent - Parent hash
   * @param {string} action - Action name
   * @param {object} params - Action parameters
   * @returns {Commit}
   */
  static action(parent, action, params = {}) {
    return new Commit({
      parent,
      type: CommitType.ACTION,
      payload: { action, params },
    });
  }

  /**
   * Create a DELETE commit
   * @param {string} parent - Parent hash
   * @param {string} path - Path to delete
   * @returns {Commit}
   */
  static delete(parent, path) {
    return new Commit({
      parent,
      type: CommitType.DELETE,
      path,
    });
  }

  /**
   * Get the canonical bytes for signing/hashing
   * @returns {Uint8Array}
   */
  canonicalBytes() {
    const obj = {
      parent: this.parent,
      type: this.type,
      path: this.path,
      payload: this.payload,
      timestamp: this.timestamp,
    };
    return new TextEncoder().encode(JSON.stringify(obj));
  }

  /**
   * Compute the commit hash
   * @returns {string}
   */
  hash() {
    if (!this._hash) {
      const bytes = this.canonicalBytes();
      this._hash = bytesToHex(sha256(bytes));
    }
    return this._hash;
  }

  /**
   * Sign this commit with an identity
   * @param {Identity} identity
   * @returns {Promise<Commit>}
   */
  async sign(identity) {
    const signature = await identity.signHex(this.canonicalBytes());
    this.signatures.push({
      publicKey: identity.publicKeyHex,
      signature,
    });
    this._hash = null; // Invalidate cached hash
    return this;
  }

  /**
   * Check if commit is signed by a specific public key
   * @param {string} publicKeyHex
   * @returns {boolean}
   */
  isSignedBy(publicKeyHex) {
    return this.signatures.some(s => s.publicKey === publicKeyHex);
  }

  /**
   * Verify all signatures
   * @param {Map<string, Identity>} identities - Map of publicKeyHex -> Identity
   * @returns {Promise<boolean>}
   */
  async verifySignatures(identities) {
    const canonical = this.canonicalBytes();
    for (const sig of this.signatures) {
      const identity = identities.get(sig.publicKey);
      if (!identity) {
        return false; // Unknown signer
      }
      const valid = await identity.verify(canonical, sig.signature);
      if (!valid) {
        return false;
      }
    }
    return true;
  }

  /**
   * Convert to JSON
   * @returns {object}
   */
  toJSON() {
    return {
      hash: this.hash(),
      parent: this.parent,
      type: this.type,
      path: this.path,
      payload: this.payload,
      timestamp: this.timestamp,
      signatures: this.signatures,
    };
  }

  /**
   * Create from JSON
   * @param {object} json
   * @returns {Commit}
   */
  static fromJSON(json) {
    const commit = new Commit({
      parent: json.parent,
      type: json.type,
      path: json.path,
      payload: json.payload,
      timestamp: json.timestamp,
      signatures: json.signatures || [],
    });
    return commit;
  }
}

export default Commit;
