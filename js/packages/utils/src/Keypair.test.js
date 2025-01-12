import { jest, expect, describe, test, it, beforeEach } from "@jest/globals";
import Keypair from '../src/Keypair.js';

describe('Keypair', () => {
  let keypair;

  beforeEach(async () => {
    keypair = await Keypair.generate();
  });

  test('should generate a valid keypair', async () => {
    expect(keypair).toBeInstanceOf(Keypair);
    expect(await keypair.publicKeyAsBase58Identity()).toEqual(expect.any(String));
  });

  test('should convert public key to multiaddr string', async () => {
    const multiaddr = await keypair.publicKeyToMultiaddrString();
    expect(multiaddr).toEqual(expect.any(String));
    expect(multiaddr).toMatch(/^\/ed25519-pub\//);
    expect(multiaddr).toHaveLength(13 + 52);
  });

  test('should serialize and deserialize keypair to/from JSON', async () => {
    const json = await keypair.asJSON();
    expect(json).toHaveProperty('id');
    expect(json).toHaveProperty('public_key');
    expect(json).toHaveProperty('private_key');

    const deserializedKeypair = await Keypair.fromJSON(json);
    expect(await deserializedKeypair.publicKeyAsBase58Identity()).toEqual(await keypair.publicKeyAsBase58Identity());
  });
  
  test('should deserialize known keypass', async () => {
    const keypass = {
      id: '12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd',
      public_key: 'CAESIAAidFtWD6boXLywUfSZJJPusMe7q+tyYYGyZxz59EGI',
      private_key: 'CAESQOXrkVunUwHzs4yfH+1e5MXyeK0PTHQMJ3Jf+Sbx4/2uACJ0W1YPpuhcvLBR9Jkkk+6wx7ur63JhgbJnHPn0QYg='
    };
    const deserializedKeypair = await Keypair.fromJSON(keypass);
    expect(await deserializedKeypair.publicKeyAsBase58Identity()).toEqual(keypass.id); 
  });

  test('should serialize public key to JSON', async () => {
    const json = await keypair.asPublicJSON();
    expect(json).toHaveProperty('id');
    expect(json).toHaveProperty('public_key');
    expect(json).not.toHaveProperty('private_key');
  });

  test('should sign and verify a message', async () => {
    const message = 'Hello, World!';
    const signature = await keypair.signStringAsBase64Pad(message);
    const isValid = await keypair.verifySignatureForString(signature, message);
    expect(isValid).toBe(true);

    const wrongMessage = 'Hello, World?';
    const isInvalid = await keypair.verifySignatureForString(signature, wrongMessage);
    expect(isInvalid).toBe(false);
  });

  test('should sign and verify JSON', async () => {
    const json = { name: 'Alice', age: 30 };
    const signature = await keypair.signJSON(json);
    const isValid = await keypair.verifyJSON(signature, json);
    expect(isValid).toBe(true);

    const wrongJson = { name: 'Alice', age: 31 };
    const isInvalid = await keypair.verifyJSON(signature, wrongJson);
    expect(isInvalid).toBe(false);
  });

  test('should create keypair from public key', async () => {
    const publicKeyId = await keypair.publicKeyAsBase58Identity();
    const publicKeypair = await Keypair.fromPublicKey(publicKeyId);
    expect(await publicKeypair.publicKeyAsBase58Identity()).toEqual(publicKeyId);
    expect(publicKeypair.key.private).toBeUndefined();
  });

  // TODO
  test.skip('should handle SSH public key conversion', async () => {
    const sshPubKey = await keypair.asSSHDotPub();
    expect(sshPubKey).toEqual(expect.any(String));
    expect(sshPubKey).toMatch(/^ssh-ed25519 /);

    const importedKeypair = await Keypair.fromSSHDotPub(sshPubKey);
    expect(await importedKeypair.publicKeyAsBase58Identity()).toEqual(await keypair.publicKeyAsBase58Identity());
  });
});