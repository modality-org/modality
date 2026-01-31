//! Cryptographic utilities for Modality contracts
//!
//! Provides ed25519 signature verification for contract predicates.

use serde::{Serialize, Deserialize};

/// Result of signature verification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerifyResult {
    Valid,
    Invalid,
    Error(String),
}

/// Verify an ed25519 signature
///
/// # Arguments
/// * `public_key` - Hex-encoded public key (64 chars = 32 bytes)
/// * `message` - The message that was signed
/// * `signature` - Hex-encoded signature (128 chars = 64 bytes)
///
/// # Returns
/// * `VerifyResult::Valid` if signature is valid
/// * `VerifyResult::Invalid` if signature is invalid
/// * `VerifyResult::Error` if inputs are malformed
#[cfg(not(target_arch = "wasm32"))]
pub fn verify_ed25519(public_key: &str, message: &[u8], signature: &str) -> VerifyResult {
    use ed25519_dalek::{Signature, VerifyingKey, Verifier};

    // Parse public key from hex
    let pubkey_bytes = match hex::decode(public_key) {
        Ok(bytes) => bytes,
        Err(e) => return VerifyResult::Error(format!("Invalid public key hex: {}", e)),
    };

    if pubkey_bytes.len() != 32 {
        return VerifyResult::Error(format!(
            "Public key must be 32 bytes, got {}",
            pubkey_bytes.len()
        ));
    }

    let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return VerifyResult::Error("Failed to convert public key".to_string()),
    };

    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(k) => k,
        Err(e) => return VerifyResult::Error(format!("Invalid public key: {}", e)),
    };

    // Parse signature from hex
    let sig_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(e) => return VerifyResult::Error(format!("Invalid signature hex: {}", e)),
    };

    if sig_bytes.len() != 64 {
        return VerifyResult::Error(format!(
            "Signature must be 64 bytes, got {}",
            sig_bytes.len()
        ));
    }

    let sig_array: [u8; 64] = match sig_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return VerifyResult::Error("Failed to convert signature".to_string()),
    };

    let signature = Signature::from_bytes(&sig_array);

    // Verify
    match verifying_key.verify(message, &signature) {
        Ok(_) => VerifyResult::Valid,
        Err(_) => VerifyResult::Invalid,
    }
}

/// WASM-compatible stub (actual crypto in JS)
#[cfg(target_arch = "wasm32")]
pub fn verify_ed25519(_public_key: &str, _message: &[u8], _signature: &str) -> VerifyResult {
    // In WASM, we delegate to JavaScript for crypto
    // This is a placeholder - real implementation would use wasm-bindgen
    VerifyResult::Error("Crypto not available in WASM mode - use JS".to_string())
}

/// Sign a message with an ed25519 private key
///
/// # Arguments
/// * `secret_key` - Hex-encoded secret key (64 chars = 32 bytes)
/// * `message` - The message to sign
///
/// # Returns
/// * `Ok(signature)` - Hex-encoded signature
/// * `Err(error)` - If signing fails
#[cfg(not(target_arch = "wasm32"))]
pub fn sign_ed25519(secret_key: &str, message: &[u8]) -> Result<String, String> {
    use ed25519_dalek::{SigningKey, Signer};

    // Parse secret key from hex
    let secret_bytes = hex::decode(secret_key)
        .map_err(|e| format!("Invalid secret key hex: {}", e))?;

    if secret_bytes.len() != 32 {
        return Err(format!(
            "Secret key must be 32 bytes, got {}",
            secret_bytes.len()
        ));
    }

    let secret_array: [u8; 32] = secret_bytes.try_into()
        .map_err(|_| "Failed to convert secret key")?;

    let signing_key = SigningKey::from_bytes(&secret_array);
    let signature = signing_key.sign(message);

    Ok(hex::encode(signature.to_bytes()))
}

/// WASM stub
#[cfg(target_arch = "wasm32")]
pub fn sign_ed25519(_secret_key: &str, _message: &[u8]) -> Result<String, String> {
    Err("Crypto not available in WASM mode - use JS".to_string())
}

/// Generate a new ed25519 keypair
///
/// # Returns
/// * `(secret_key_hex, public_key_hex)` tuple
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_keypair() -> (String, String) {
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    (
        hex::encode(signing_key.to_bytes()),
        hex::encode(verifying_key.to_bytes()),
    )
}

/// WASM stub
#[cfg(target_arch = "wasm32")]
pub fn generate_keypair() -> (String, String) {
    ("".to_string(), "".to_string())
}

/// Hash a message using SHA-256
#[cfg(not(target_arch = "wasm32"))]
pub fn sha256(message: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(message);
    hex::encode(hasher.finalize())
}

/// WASM stub
#[cfg(target_arch = "wasm32")]
pub fn sha256(_message: &[u8]) -> String {
    "".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_generate_and_sign() {
        let (secret, public) = generate_keypair();
        
        assert_eq!(secret.len(), 64); // 32 bytes hex
        assert_eq!(public.len(), 64); // 32 bytes hex

        let message = b"Hello, Modality!";
        let signature = sign_ed25519(&secret, message).unwrap();
        
        assert_eq!(signature.len(), 128); // 64 bytes hex

        let result = verify_ed25519(&public, message, &signature);
        assert_eq!(result, VerifyResult::Valid);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_invalid_signature() {
        let (_, public) = generate_keypair();
        let message = b"Hello, Modality!";
        
        // Wrong signature
        let bad_sig = "00".repeat(64);
        let result = verify_ed25519(&public, message, &bad_sig);
        assert_eq!(result, VerifyResult::Invalid);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_wrong_message() {
        let (secret, public) = generate_keypair();
        
        let message1 = b"Hello, Modality!";
        let message2 = b"Different message";
        
        let signature = sign_ed25519(&secret, message1).unwrap();
        
        // Signature for message1 should not verify for message2
        let result = verify_ed25519(&public, message2, &signature);
        assert_eq!(result, VerifyResult::Invalid);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_sha256() {
        let hash = sha256(b"hello");
        assert_eq!(hash.len(), 64); // 32 bytes hex
        // Known hash for "hello"
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
