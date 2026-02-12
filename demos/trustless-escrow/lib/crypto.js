/**
 * Ed25519 Cryptography for Demo
 * 
 * Uses noble-ed25519 for real signatures in the browser.
 * https://github.com/paulmillr/noble-ed25519
 */

// Import noble-ed25519 from CDN (ESM)
const ed25519Module = await import('https://esm.sh/@noble/ed25519@2.1.0');
const ed = ed25519Module;

// Use webcrypto for randomness
ed.etc.sha512Sync = undefined;
ed.etc.sha512Async = async (message) => {
  const hashBuffer = await crypto.subtle.digest('SHA-512', message);
  return new Uint8Array(hashBuffer);
};

/**
 * Generate a new ed25519 keypair
 */
export async function generateKeypair() {
  const privateKey = ed.utils.randomPrivateKey();
  const publicKey = await ed.getPublicKeyAsync(privateKey);
  
  return {
    privateKey: bytesToHex(privateKey),
    publicKey: bytesToHex(publicKey)
  };
}

/**
 * Sign a message with a private key
 */
export async function sign(privateKeyHex, message) {
  const privateKey = hexToBytes(privateKeyHex);
  const messageBytes = typeof message === 'string' 
    ? new TextEncoder().encode(message)
    : message;
  
  const signature = await ed.signAsync(messageBytes, privateKey);
  return bytesToHex(signature);
}

/**
 * Verify a signature
 */
export async function verify(publicKeyHex, message, signatureHex) {
  const publicKey = hexToBytes(publicKeyHex);
  const signature = hexToBytes(signatureHex);
  const messageBytes = typeof message === 'string'
    ? new TextEncoder().encode(message)
    : message;
  
  return await ed.verifyAsync(signature, messageBytes, publicKey);
}

/**
 * Create a signed commit
 */
export async function signCommit(privateKeyHex, publicKeyHex, commit) {
  const canonical = JSON.stringify(commit, Object.keys(commit).sort());
  const signature = await sign(privateKeyHex, canonical);
  
  return {
    ...commit,
    signatures: [{
      signer: publicKeyHex,
      signature
    }]
  };
}

/**
 * Verify a signed commit
 */
export async function verifyCommit(commit) {
  if (!commit.signatures || commit.signatures.length === 0) {
    return false;
  }
  
  // Remove signatures for canonical form
  const { signatures, ...commitBody } = commit;
  const canonical = JSON.stringify(commitBody, Object.keys(commitBody).sort());
  
  for (const sig of signatures) {
    const valid = await verify(sig.signer, canonical, sig.signature);
    if (!valid) return false;
  }
  
  return true;
}

/**
 * Hash data using SHA-256
 */
export async function sha256(data) {
  const bytes = typeof data === 'string'
    ? new TextEncoder().encode(data)
    : data;
  
  const hashBuffer = await crypto.subtle.digest('SHA-256', bytes);
  return bytesToHex(new Uint8Array(hashBuffer));
}

// Utility functions
function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

export default {
  generateKeypair,
  sign,
  verify,
  signCommit,
  verifyCommit,
  sha256
};
