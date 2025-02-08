const str2bytes = (str) => new TextEncoder().encode(str);
const bytes2str = (bytes) => new TextDecoder().decode(bytes);
const getRandomValues = (size) => crypto.getRandomValues(new Uint8Array(size));

// Derive a key from a password using PBKDF2 and HKDF
async function passwordToKey(password, salt) {
  // First, create a base key from the password
  const baseKey = await crypto.subtle.importKey(
    "raw",
    str2bytes(password),
    "PBKDF2",
    false,
    ["deriveBits"]
  );

  // Derive bits using PBKDF2
  const iterations = 100000;
  const keyMaterial = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      salt: salt,
      iterations: iterations,
      hash: "SHA-256",
    },
    baseKey,
    256 // 32 bytes for AES-256
  );

  // Use HKDF to derive the final key
  const hkdfKey = await crypto.subtle.importKey(
    "raw",
    keyMaterial,
    { name: "HKDF" },
    false,
    ["deriveBits"]
  );

  const finalKeyBits = await crypto.subtle.deriveBits(
    {
      name: "HKDF",
      hash: "SHA-256",
      salt: salt,
      info: str2bytes("aes-256-gcm"),
    },
    hkdfKey,
    256
  );

  // Import as AES-GCM key
  return await crypto.subtle.importKey(
    "raw",
    finalKeyBits,
    { name: "AES-GCM" },
    false,
    ["encrypt", "decrypt"]
  );
}

export async function encrypt(text, password) {
  try {
    // Generate a random salt
    const salt = getRandomValues(16);

    // Generate a random nonce for AES-GCM
    const nonce = getRandomValues(12); // 96 bits for GCM

    // Derive key from password
    const key = await passwordToKey(password, salt);

    // Encrypt the text
    const encryptedData = await crypto.subtle.encrypt(
      {
        name: "AES-GCM",
        iv: nonce,
        tagLength: 128, // 16 bytes authentication tag
      },
      key,
      str2bytes(text)
    );

    // Combine salt, nonce, and encrypted data (includes auth tag)
    const combined = new Uint8Array(
      salt.length + nonce.length + encryptedData.byteLength
    );
    combined.set(salt);
    combined.set(nonce, salt.length);
    combined.set(new Uint8Array(encryptedData), salt.length + nonce.length);

    // Convert to base64
    return btoa(String.fromCharCode(...combined));
  } catch (error) {
    throw new Error(`Encryption failed: ${error.message}`);
  }
}

export async function decrypt(encryptedBase64, password) {
  try {
    // Convert from base64
    const combined = new Uint8Array(
      atob(encryptedBase64)
        .split("")
        .map((c) => c.charCodeAt(0))
    );

    if (combined.length < 28) {
      // 16 (salt) + 12 (nonce)
      throw new Error("Data too short");
    }

    // Extract components
    const salt = combined.slice(0, 16);
    const nonce = combined.slice(16, 28);
    const encryptedData = combined.slice(28);

    // Derive the same key from password
    const key = await passwordToKey(password, salt);

    // Decrypt the data
    const decryptedData = await crypto.subtle.decrypt(
      {
        name: "AES-GCM",
        iv: nonce,
        tagLength: 128,
      },
      key,
      encryptedData
    );

    return bytes2str(new Uint8Array(decryptedData));
  } catch (error) {
    throw new Error("Decryption failed - invalid password or corrupted data");
  }
}
