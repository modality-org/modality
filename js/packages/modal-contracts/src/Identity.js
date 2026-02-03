/**
 * Identity - Ed25519 keypair for signing commits
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils';

// Configure ed25519 to use sha512
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

/**
 * Represents an identity (Ed25519 keypair)
 */
export class Identity {
  /**
   * @param {Uint8Array} privateKey - 32-byte private key
   * @param {Uint8Array} publicKey - 32-byte public key
   */
  constructor(privateKey, publicKey) {
    this._privateKey = privateKey;
    this._publicKey = publicKey;
  }

  /**
   * Generate a new random identity
   * @returns {Promise<Identity>}
   */
  static async generate() {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    return new Identity(privateKey, publicKey);
  }

  /**
   * Create identity from hex-encoded private key
   * @param {string} privateKeyHex
   * @returns {Promise<Identity>}
   */
  static async fromPrivateKey(privateKeyHex) {
    const privateKey = hexToBytes(privateKeyHex);
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    return new Identity(privateKey, publicKey);
  }

  /**
   * Create identity from public key only (for verification)
   * @param {string} publicKeyHex
   * @returns {Identity}
   */
  static fromPublicKey(publicKeyHex) {
    const publicKey = hexToBytes(publicKeyHex);
    return new Identity(null, publicKey);
  }

  /**
   * Get the public key as hex string
   * @returns {string}
   */
  get publicKeyHex() {
    return bytesToHex(this._publicKey);
  }

  /**
   * Get the private key as hex string (if available)
   * @returns {string|null}
   */
  get privateKeyHex() {
    return this._privateKey ? bytesToHex(this._privateKey) : null;
  }

  /**
   * Check if this identity can sign (has private key)
   * @returns {boolean}
   */
  canSign() {
    return this._privateKey !== null;
  }

  /**
   * Sign a message
   * @param {Uint8Array|string} message
   * @returns {Promise<Uint8Array>}
   */
  async sign(message) {
    if (!this._privateKey) {
      throw new Error('Cannot sign: no private key');
    }
    const msgBytes = typeof message === 'string' 
      ? new TextEncoder().encode(message) 
      : message;
    return ed.signAsync(msgBytes, this._privateKey);
  }

  /**
   * Sign a message and return hex-encoded signature
   * @param {Uint8Array|string} message
   * @returns {Promise<string>}
   */
  async signHex(message) {
    const sig = await this.sign(message);
    return bytesToHex(sig);
  }

  /**
   * Verify a signature
   * @param {Uint8Array|string} message
   * @param {Uint8Array|string} signature
   * @returns {Promise<boolean>}
   */
  async verify(message, signature) {
    const msgBytes = typeof message === 'string' 
      ? new TextEncoder().encode(message) 
      : message;
    const sigBytes = typeof signature === 'string'
      ? hexToBytes(signature)
      : signature;
    return ed.verifyAsync(sigBytes, msgBytes, this._publicKey);
  }

  /**
   * Export to JSON (includes private key if available)
   * @returns {object}
   */
  toJSON() {
    return {
      publicKey: this.publicKeyHex,
      privateKey: this.privateKeyHex,
    };
  }

  /**
   * Create identity from JSON
   * @param {object} json
   * @returns {Promise<Identity>}
   */
  static async fromJSON(json) {
    if (json.privateKey) {
      return Identity.fromPrivateKey(json.privateKey);
    }
    return Identity.fromPublicKey(json.publicKey);
  }
}

export default Identity;
