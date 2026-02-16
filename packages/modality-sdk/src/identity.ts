import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils';

// ed25519 requires sha512 sync
ed.etc.sha512Sync = (...m: Uint8Array[]) => {
  const h = sha512.create();
  for (const msg of m) h.update(msg);
  return h.digest();
};

export interface Identity {
  privateKey: string; // hex
  publicKey: string;  // hex
}

/** Generate a new Ed25519 keypair */
export function generateIdentity(): Identity {
  const privateKey = ed.utils.randomPrivateKey();
  const publicKey = ed.getPublicKey(privateKey);
  return {
    privateKey: bytesToHex(privateKey),
    publicKey: bytesToHex(publicKey),
  };
}

/** Restore identity from a hex private key */
export function identityFromPrivateKey(privateKeyHex: string): Identity {
  const publicKey = ed.getPublicKey(hexToBytes(privateKeyHex));
  return {
    privateKey: privateKeyHex,
    publicKey: bytesToHex(publicKey),
  };
}

/** Sign arbitrary bytes, returns hex signature */
export function sign(message: Uint8Array, privateKeyHex: string): string {
  const sig = ed.sign(message, hexToBytes(privateKeyHex));
  return bytesToHex(sig);
}

/** Verify a signature */
export function verify(
  message: Uint8Array,
  signatureHex: string,
  publicKeyHex: string,
): boolean {
  return ed.verify(hexToBytes(signatureHex), message, hexToBytes(publicKeyHex));
}

/** Sign a JSON object (canonical serialization) */
export function signJSON(data: unknown, privateKeyHex: string): string {
  const bytes = new TextEncoder().encode(JSON.stringify(data));
  return sign(bytes, privateKeyHex);
}

/** Verify a JSON signature */
export function verifyJSON(
  data: unknown,
  signatureHex: string,
  publicKeyHex: string,
): boolean {
  const bytes = new TextEncoder().encode(JSON.stringify(data));
  return verify(bytes, signatureHex, publicKeyHex);
}
