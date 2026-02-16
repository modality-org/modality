import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils';
// ed25519 requires sha512 sync
ed.etc.sha512Sync = (...m) => {
    const h = sha512.create();
    for (const msg of m)
        h.update(msg);
    return h.digest();
};
/** Generate a new Ed25519 keypair */
export function generateIdentity() {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = ed.getPublicKey(privateKey);
    return {
        privateKey: bytesToHex(privateKey),
        publicKey: bytesToHex(publicKey),
    };
}
/** Restore identity from a hex private key */
export function identityFromPrivateKey(privateKeyHex) {
    const publicKey = ed.getPublicKey(hexToBytes(privateKeyHex));
    return {
        privateKey: privateKeyHex,
        publicKey: bytesToHex(publicKey),
    };
}
/** Sign arbitrary bytes, returns hex signature */
export function sign(message, privateKeyHex) {
    const sig = ed.sign(message, hexToBytes(privateKeyHex));
    return bytesToHex(sig);
}
/** Verify a signature */
export function verify(message, signatureHex, publicKeyHex) {
    return ed.verify(hexToBytes(signatureHex), message, hexToBytes(publicKeyHex));
}
/** Sign a JSON object (canonical serialization) */
export function signJSON(data, privateKeyHex) {
    const bytes = new TextEncoder().encode(JSON.stringify(data));
    return sign(bytes, privateKeyHex);
}
/** Verify a JSON signature */
export function verifyJSON(data, signatureHex, publicKeyHex) {
    const bytes = new TextEncoder().encode(JSON.stringify(data));
    return verify(bytes, signatureHex, publicKeyHex);
}
