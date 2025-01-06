use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ring::{aead, pbkdf2, rand, hkdf};
use ring::rand::SecureRandom;
use std::num::NonZeroU32;

pub struct EncryptedText;

impl EncryptedText {
    fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], &'static str> {
        const ITERATIONS: u32 = 100_000;
        let iterations = NonZeroU32::new(ITERATIONS).unwrap();
        let mut pbkdf2_output = [0u8; 32];

        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            iterations,
            salt,
            password.as_bytes(),
            &mut pbkdf2_output,
        );

        // Then HKDF
        let hkdf_salt = hkdf::Salt::new(hkdf::HKDF_SHA256, salt);
        let prk = hkdf_salt.extract(&pbkdf2_output);
        let mut final_key = [0u8; 32];
        
        // Use expand and fill with a fixed length
        prk.expand(&[b"aes-256-gcm"], hkdf::HKDF_SHA256)
            .map_err(|_| "HKDF expand failed")?
            .fill(&mut final_key)
            .map_err(|_| "HKDF fill failed")?;


        Ok(final_key)
    }

    pub fn encrypt(text: &str, password: &str) -> Result<String, &'static str> {
        let rng = rand::SystemRandom::new();
        let mut salt = vec![0u8; 16];
        rng.fill(&mut salt).map_err(|_| "Failed to generate salt")?;

        let key_bytes = Self::derive_key(password, &salt)?;

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
            .map_err(|_| "Failed to create key")?;
        let key = aead::LessSafeKey::new(unbound_key);

        let mut nonce_bytes = vec![0u8; 12];
        rng.fill(&mut nonce_bytes).map_err(|_| "Failed to generate nonce")?;
        let nonce = aead::Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| "Invalid nonce")?;

        let mut in_out = text.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
            .map_err(|_| "Encryption failed")?;

        let mut combined = Vec::new();
        combined.extend_from_slice(&salt);
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&in_out);

        Ok(BASE64.encode(combined))
    }

    pub fn decrypt(encrypted_base64: &str, password: &str) -> Result<String, &'static str> {
        let combined = BASE64.decode(encrypted_base64)
            .map_err(|_| "Invalid base64 data")?;

        if combined.len() < 28 {
            return Err("Data too short");
        }

        let salt = &combined[0..16];
        let nonce_bytes = &combined[16..28];
        let ciphertext = &combined[28..];

        let key_bytes = Self::derive_key(password, salt)?;

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
            .map_err(|_| "Failed to create key")?;
        let key = aead::LessSafeKey::new(unbound_key);

        let nonce = aead::Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|_| "Invalid nonce")?;

        let mut decrypted = ciphertext.to_vec();
        let decrypted_len = key.open_in_place(nonce, aead::Aad::empty(), &mut decrypted)
            .map_err(|_| "Decryption failed - invalid password or corrupted data")?
            .len();

        // Truncate to the actual decrypted length (excluding auth tag)
        decrypted.truncate(decrypted_len);

        String::from_utf8(decrypted)
            .map_err(|_| "Invalid UTF-8 in decrypted data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let password = "MySecretPassword123!";
        let text = "Hello, Web Crypto with password-based encryption!";

        let encrypted = EncryptedText::encrypt(text, password).unwrap();
        let decrypted = EncryptedText::decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, text);

        let result = EncryptedText::decrypt(&encrypted, "WrongPassword");
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupted_data() {
        let password = "MySecretPassword123!";
        let text = "Test message";

        let encrypted = EncryptedText::encrypt(text, password).unwrap();
        
        // Corrupt different parts of the data
        let corrupted_start = format!("A{}", &encrypted[1..]);
        assert!(EncryptedText::decrypt(&corrupted_start, password).is_err());

        let mid = encrypted.len() / 2;
        let corrupted_middle = format!("{}A{}", &encrypted[..mid], &encrypted[mid+1..]);
        assert!(EncryptedText::decrypt(&corrupted_middle, password).is_err());

        let corrupted_end = format!("{}AAAA", &encrypted[..encrypted.len()-4]);
        assert!(EncryptedText::decrypt(&corrupted_end, password).is_err());
    }

    #[test]
    fn test_various_lengths() {
        let password = "test123";
        let texts = vec![
            "",
            "a",
            "hello",
            "This is a longer test message with spaces and !@#$ symbols",
            "🦀 Rust with Unicode 🔐"
        ];

        for text in texts {
            let encrypted = EncryptedText::encrypt(text, password).unwrap();
            let decrypted = EncryptedText::decrypt(&encrypted, password).unwrap();
            assert_eq!(text, decrypted);
        }
    }
    
    #[test]
    fn test_known_string() {
        const KNOWN_PASSWORD: &str = "test_password_123";
        const KNOWN_MESSAGE: &str = "Hello, Cross-Platform Encryption!";
        const KNOWN_ENCRYPTED: &str = "1G73otj9BTJ5i3djZyuemijZnGkMb8XawInJVUqLqiNTIRPrBrs8MxL0y+cJWTcxGcxkS7H+/BltKwxqS0dd5TYTN81cOWaHmO7SJR0=";
    
        // Test decryption of known string
        let decrypted = EncryptedText::decrypt(KNOWN_ENCRYPTED, KNOWN_PASSWORD).unwrap();
        assert_eq!(decrypted, KNOWN_MESSAGE);

        // Test that we can also encrypt and decrypt our own message
        let encrypted = EncryptedText::encrypt(KNOWN_MESSAGE, KNOWN_PASSWORD).unwrap();
        let decrypted = EncryptedText::decrypt(&encrypted, KNOWN_PASSWORD).unwrap();
        assert_eq!(decrypted, KNOWN_MESSAGE);
    }
}