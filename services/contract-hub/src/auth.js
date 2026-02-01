/**
 * Authentication via ed25519 signatures
 * 
 * Client signs: METHOD + PATH + TIMESTAMP + BODY_HASH
 * Server verifies signature using registered public key
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Configure ed25519 to use sha512
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export class AuthMiddleware {
  constructor(store) {
    this.store = store;
  }
  
  /**
   * Create verification middleware
   * Expects headers:
   *   X-Access-Id: <access_id>
   *   X-Timestamp: <unix_timestamp_ms>
   *   X-Signature: <hex_signature>
   */
  verify() {
    return async (req, res, next) => {
      try {
        const accessId = req.headers['x-access-id'];
        const timestamp = req.headers['x-timestamp'];
        const signature = req.headers['x-signature'];
        
        if (!accessId || !timestamp || !signature) {
          return res.status(401).json({ 
            error: 'Missing auth headers',
            required: ['X-Access-Id', 'X-Timestamp', 'X-Signature']
          });
        }
        
        // Check timestamp (within 5 minutes)
        const ts = parseInt(timestamp, 10);
        const now = Date.now();
        if (isNaN(ts) || Math.abs(now - ts) > 5 * 60 * 1000) {
          return res.status(401).json({ error: 'Invalid or expired timestamp' });
        }
        
        // Get public key for access ID
        const accessInfo = this.store.getAccess(accessId);
        if (!accessInfo) {
          return res.status(401).json({ error: 'Unknown access ID' });
        }
        
        // Build message to verify
        const bodyHash = await this.hashBody(req.body);
        const message = `${req.method}:${req.originalUrl}:${timestamp}:${bodyHash}`;
        const messageBytes = new TextEncoder().encode(message);
        
        // Verify signature
        const sigBytes = hexToBytes(signature);
        const pubkeyBytes = hexToBytes(accessInfo.public_key);
        
        const valid = await ed.verifyAsync(sigBytes, messageBytes, pubkeyBytes);
        if (!valid) {
          return res.status(401).json({ error: 'Invalid signature' });
        }
        
        // Attach access ID and identity ID to request
        req.accessId = accessId;
        req.accessInfo = accessInfo;
        req.identityId = accessInfo.identity_id;
        
        next();
      } catch (err) {
        return res.status(401).json({ error: 'Auth failed: ' + err.message });
      }
    };
  }
  
  async hashBody(body) {
    if (!body || (typeof body === 'object' && Object.keys(body).length === 0)) {
      return 'empty';
    }
    const str = typeof body === 'string' ? body : JSON.stringify(body);
    const bytes = new TextEncoder().encode(str);
    const hash = sha512(bytes);
    return bytesToHex(hash.slice(0, 16)); // First 16 bytes as hex
  }
  
  /**
   * Verify a signature against a public key
   */
  async verifySignature(publicKeyHex, message, signatureHex) {
    try {
      const messageBytes = new TextEncoder().encode(message);
      const sigBytes = hexToBytes(signatureHex);
      const pubkeyBytes = hexToBytes(publicKeyHex);
      return await ed.verifyAsync(sigBytes, messageBytes, pubkeyBytes);
    } catch {
      return false;
    }
  }
}

/**
 * Client-side helper to sign requests
 */
export async function signRequest(privateKeyHex, method, path, timestamp, body) {
  const bodyHash = await hashBodyClient(body);
  const message = `${method}:${path}:${timestamp}:${bodyHash}`;
  const messageBytes = new TextEncoder().encode(message);
  const privateKey = hexToBytes(privateKeyHex);
  const signature = await ed.signAsync(messageBytes, privateKey);
  return bytesToHex(signature);
}

async function hashBodyClient(body) {
  if (!body || (typeof body === 'object' && Object.keys(body).length === 0)) {
    return 'empty';
  }
  const str = typeof body === 'string' ? body : JSON.stringify(body);
  const bytes = new TextEncoder().encode(str);
  const hash = sha512(bytes);
  return bytesToHex(hash.slice(0, 16));
}

// Hex utilities
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
