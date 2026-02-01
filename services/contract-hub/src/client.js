/**
 * Contract Hub Client
 * 
 * Simple client for interacting with the Contract Hub service.
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Configure ed25519 to use sha512
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export class ContractHubClient {
  constructor(baseUrl, { accessId, privateKey } = {}) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.accessId = accessId;
    this.privateKey = privateKey;
  }
  
  // ============================================================================
  // KEY MANAGEMENT
  // ============================================================================
  
  /**
   * Generate a new keypair
   * Returns { privateKey, publicKey } as hex strings
   */
  static async generateKeypair() {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    return {
      privateKey: bytesToHex(privateKey),
      publicKey: bytesToHex(publicKey)
    };
  }
  
  /**
   * Get public key from private key
   */
  static async getPublicKey(privateKeyHex) {
    const privateKey = hexToBytes(privateKeyHex);
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    return bytesToHex(publicKey);
  }
  
  // ============================================================================
  // IDENTITY REGISTRATION
  // ============================================================================
  
  /**
   * Register an identity (long-term key)
   */
  async registerIdentity(publicKeyHex) {
    return this.request('POST', '/identity/register', {
      public_key: publicKeyHex
    }, false);
  }
  
  /**
   * Create an access key for an identity
   * @param identityId - The identity to create access for
   * @param accessPublicKey - Public key for the new access key
   * @param identityPrivateKey - Identity private key to sign the request
   * @param options - { name?, expiresAt? }
   */
  async createAccessKey(identityId, accessPublicKey, identityPrivateKey, options = {}) {
    const timestamp = Date.now().toString();
    const message = `create_access:${accessPublicKey}:${timestamp}`;
    const messageBytes = new TextEncoder().encode(message);
    const privateKey = hexToBytes(identityPrivateKey);
    const signature = await ed.signAsync(messageBytes, privateKey);
    
    return this.request('POST', '/access/create', {
      identity_id: identityId,
      access_public_key: accessPublicKey,
      timestamp,
      signature: bytesToHex(signature),
      name: options.name,
      expires_at: options.expiresAt
    }, false);
  }
  
  /**
   * List access keys (requires auth)
   */
  async listAccessKeys() {
    return this.request('GET', '/access/list');
  }
  
  /**
   * Revoke an access key
   */
  async revokeAccessKey(accessId) {
    return this.request('POST', '/access/revoke', { access_id: accessId });
  }
  
  // ============================================================================
  // CONTRACT OPERATIONS
  // ============================================================================
  
  /**
   * Create a new contract
   */
  async createContract(name, description) {
    return this.request('POST', '/contracts', { name, description });
  }
  
  /**
   * List contracts owned by current user
   */
  async listContracts() {
    return this.request('GET', '/contracts');
  }
  
  /**
   * Get contract info
   */
  async getContract(contractId) {
    return this.request('GET', `/contracts/${contractId}`);
  }
  
  /**
   * Push commits to a contract
   */
  async push(contractId, commits) {
    return this.request('POST', `/contracts/${contractId}/push`, { commits });
  }
  
  /**
   * Pull commits from a contract
   */
  async pull(contractId, sinceHash = null) {
    let path = `/contracts/${contractId}/pull`;
    if (sinceHash) {
      path += `?since=${sinceHash}`;
    }
    return this.request('GET', path);
  }
  
  /**
   * Get a specific commit
   */
  async getCommit(contractId, hash) {
    return this.request('GET', `/contracts/${contractId}/commits/${hash}`);
  }
  
  /**
   * Grant access to another identity
   */
  async grantAccess(contractId, identityId, permission) {
    return this.request('POST', `/contracts/${contractId}/access`, {
      identity_id: identityId,
      permission
    });
  }
  
  // ============================================================================
  // HTTP REQUEST HANDLING
  // ============================================================================
  
  async request(method, path, body = null, requireAuth = true) {
    const url = this.baseUrl + path;
    const headers = {
      'Content-Type': 'application/json'
    };
    
    if (requireAuth) {
      if (!this.accessId || !this.privateKey) {
        throw new Error('Authentication required: set accessId and privateKey');
      }
      
      const timestamp = Date.now().toString();
      const signature = await this.sign(method, path, timestamp, body);
      
      headers['X-Access-Id'] = this.accessId;
      headers['X-Timestamp'] = timestamp;
      headers['X-Signature'] = signature;
    }
    
    const options = {
      method,
      headers
    };
    
    if (body && (method === 'POST' || method === 'PUT' || method === 'PATCH')) {
      options.body = JSON.stringify(body);
    }
    
    const res = await fetch(url, options);
    const data = await res.json();
    
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }
    
    return data;
  }
  
  async sign(method, path, timestamp, body) {
    const bodyHash = await this.hashBody(body);
    const message = `${method}:${path}:${timestamp}:${bodyHash}`;
    const messageBytes = new TextEncoder().encode(message);
    const privateKey = hexToBytes(this.privateKey);
    const signature = await ed.signAsync(messageBytes, privateKey);
    return bytesToHex(signature);
  }
  
  async hashBody(body) {
    if (!body || (typeof body === 'object' && Object.keys(body).length === 0)) {
      return 'empty';
    }
    const str = typeof body === 'string' ? body : JSON.stringify(body);
    const bytes = new TextEncoder().encode(str);
    const hash = sha512(bytes);
    return bytesToHex(hash.slice(0, 16));
  }
}

// ============================================================================
// UTILITIES
// ============================================================================

function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

export { hexToBytes, bytesToHex };
