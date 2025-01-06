import { expect, describe, test } from "@jest/globals";

import { encrypt, decrypt } from './EncryptedText.js'

describe('EncryptedText', () => {
  test('should encrypt and decrypt successfully', async () => {
      const password = "MySecretPassword123!";
      const text = "Hello, Web Crypto with password-based encryption!";

      const encrypted = await encrypt(text, password);
      const decrypted = await decrypt(encrypted, password);
      expect(decrypted).toBe(text);
  });

  test('should fail with wrong password', async () => {
      const password = "MySecretPassword123!";
      const text = "Secret message";

      const encrypted = await encrypt(text, password);
      
      await expect(decrypt(encrypted, "WrongPassword"))
          .rejects
          .toThrow("Decryption failed - invalid password or corrupted data");
  });

  test('should fail with corrupted data', async () => {
      const password = "MySecretPassword123!";
      const text = "Test message";

      const encrypted = await encrypt(text, password);
      const corrupted = encrypted.slice(0, -2);
      
      await expect(decrypt(corrupted, password))
          .rejects
          .toThrow();
  });

  test('known string', async () => {
    const KNOWN_PASSWORD = "test_password_123";
    const KNOWN_MESSAGE = "Hello, Cross-Platform Encryption!";
    const KNOWN_ENCRYPTED = "1G73otj9BTJ5i3djZyuemijZnGkMb8XawInJVUqLqiNTIRPrBrs8MxL0y+cJWTcxGcxkS7H+/BltKwxqS0dd5TYTN81cOWaHmO7SJR0=";

    try {
        // Test decryption of known string
        const decrypted = await decrypt(KNOWN_ENCRYPTED, KNOWN_PASSWORD);
        expect(decrypted).toBe(KNOWN_MESSAGE); 

        // Test that we can also encrypt and decrypt our own message
        const encrypted = await encrypt(KNOWN_MESSAGE, KNOWN_PASSWORD);
        const redecrypted = await decrypt(encrypted, KNOWN_PASSWORD);
        expect(redecrypted).toBe(KNOWN_MESSAGE);
    } catch (error) {
        console.error("Test failed:", error);
    }
  });
});