export interface Identity {
    privateKey: string;
    publicKey: string;
}
/** Generate a new Ed25519 keypair */
export declare function generateIdentity(): Identity;
/** Restore identity from a hex private key */
export declare function identityFromPrivateKey(privateKeyHex: string): Identity;
/** Sign arbitrary bytes, returns hex signature */
export declare function sign(message: Uint8Array, privateKeyHex: string): string;
/** Verify a signature */
export declare function verify(message: Uint8Array, signatureHex: string, publicKeyHex: string): boolean;
/** Sign a JSON object (canonical serialization) */
export declare function signJSON(data: unknown, privateKeyHex: string): string;
/** Verify a JSON signature */
export declare function verifyJSON(data: unknown, signatureHex: string, publicKeyHex: string): boolean;
