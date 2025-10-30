use anyhow::{anyhow, Result};
use bip39::{Language, Mnemonic as Bip39Mnemonic};
use hmac::Hmac;
use sha2::Sha512;
use zeroize::Zeroizing;

/// BIP44 derivation path for Modality
/// Using coin type 177017 for Modality
/// Format: m/44'/177017'/account'/change'/index'
pub const MODALITY_BIP44_PURPOSE: u32 = 44;
pub const MODALITY_BIP44_COIN_TYPE: u32 = 177017;
pub const MODALITY_BIP44_ACCOUNT: u32 = 0;
pub const MODALITY_BIP44_CHANGE: u32 = 0;

#[derive(Clone)]
pub struct Mnemonic {
    inner: Bip39Mnemonic,
}

impl Mnemonic {
    /// Generate a new random mnemonic with the specified word count
    /// Valid word counts: 12, 15, 18, 21, 24
    pub fn generate(word_count: usize) -> Result<Self> {
        use rand::RngCore;
        use rand::rngs::OsRng;
        
        // Calculate entropy length based on word count
        // 12 words = 128 bits = 16 bytes
        // 15 words = 160 bits = 20 bytes
        // 18 words = 192 bits = 24 bytes
        // 21 words = 224 bits = 28 bytes
        // 24 words = 256 bits = 32 bytes
        let entropy_len = match word_count {
            12 => 16,
            15 => 20,
            18 => 24,
            21 => 28,
            24 => 32,
            _ => {
                return Err(anyhow!(
                    "Invalid word count: {}. Must be 12, 15, 18, 21, or 24",
                    word_count
                ))
            }
        };

        let mut entropy = vec![0u8; entropy_len];
        OsRng.fill_bytes(&mut entropy);

        let mnemonic = Bip39Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| anyhow!("Failed to generate mnemonic: {:?}", e))?;

        Ok(Self { inner: mnemonic })
    }

    /// Create a mnemonic from a phrase string
    pub fn from_phrase(phrase: &str) -> Result<Self> {
        let mnemonic = Bip39Mnemonic::parse_in(Language::English, phrase)
            .map_err(|e| anyhow!("Invalid mnemonic phrase: {:?}", e))?;
        Ok(Self { inner: mnemonic })
    }

    /// Get the mnemonic phrase as a string
    pub fn phrase(&self) -> String {
        self.inner.words().collect::<Vec<&str>>().join(" ")
    }

    /// Get the mnemonic word count
    pub fn word_count(&self) -> usize {
        self.inner.word_count()
    }

    /// Derive a seed from the mnemonic with an optional passphrase
    /// This implements BIP39 seed derivation
    pub fn to_seed(&self, passphrase: Option<&str>) -> Zeroizing<[u8; 64]> {
        let passphrase = passphrase.unwrap_or("");
        let seed_bytes = self.inner.to_seed(passphrase);
        let mut seed = Zeroizing::new([0u8; 64]);
        seed.copy_from_slice(&seed_bytes);
        seed
    }

    /// Derive an Ed25519 keypair at a specific BIP44 path
    /// Path format: m/44'/501'/account'/change'/index'
    pub fn derive_ed25519_keypair(
        &self,
        account: u32,
        change: u32,
        index: u32,
        passphrase: Option<&str>,
    ) -> Result<ed25519_dalek::Keypair> {
        let seed = self.to_seed(passphrase);
        
        // Derive using BIP44 path: m/44'/501'/account'/change'/index'
        let path = format!(
            "m/{}'/{}'/{}'/{}'/{}'",
            MODALITY_BIP44_PURPOSE,
            MODALITY_BIP44_COIN_TYPE,
            account,
            change,
            index
        );

        // For Ed25519, we use SLIP-0010 derivation
        let derived_key = derive_ed25519_from_seed(&seed, &path)?;
        
        let secret = ed25519_dalek::SecretKey::from_bytes(&derived_key)
            .map_err(|e| anyhow!("Failed to create Ed25519 secret key: {}", e))?;
        
        let public = ed25519_dalek::PublicKey::from(&secret);
        
        Ok(ed25519_dalek::Keypair { secret, public })
    }

    /// Get the default derivation path string
    pub fn default_derivation_path(account: u32, change: u32, index: u32) -> String {
        format!(
            "m/{}'/{}'/{}'/{}'/{}'",
            MODALITY_BIP44_PURPOSE,
            MODALITY_BIP44_COIN_TYPE,
            account,
            change,
            index
        )
    }
}

/// Derive an Ed25519 key from a seed using SLIP-0010 derivation
/// This is a simplified implementation for Ed25519 derivation
fn derive_ed25519_from_seed(seed: &[u8; 64], path: &str) -> Result<[u8; 32]> {
    // Parse the path
    let components: Vec<&str> = path.trim_start_matches("m/").split('/').collect();
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&seed[0..32]);
    
    // For each component in the path
    for component in components {
        let component = component.trim_end_matches('\'');
        let index: u32 = component
            .parse()
            .map_err(|_| anyhow!("Invalid path component: {}", component))?;
        
        // For hardened derivation (indicated by ')
        let hardened_index = index | 0x80000000;
        
        // Use HMAC-SHA512 for derivation
        let mut hmac =
            Hmac::<Sha512>::new_from_slice(b"ed25519 seed")
                .map_err(|e| anyhow!("Failed to create HMAC: {}", e))?;
        
        use hmac::Mac;
        hmac.update(&[0u8]); // 0x00 for hardened
        hmac.update(&key);
        hmac.update(&hardened_index.to_be_bytes());
        
        let result = hmac.finalize().into_bytes();
        key.copy_from_slice(&result[0..32]);
    }
    
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic() {
        let mnemonic = Mnemonic::generate(12).unwrap();
        assert_eq!(mnemonic.word_count(), 12);
        
        let phrase = mnemonic.phrase();
        assert!(phrase.split_whitespace().count() == 12);
    }

    #[test]
    fn test_from_phrase() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::from_phrase(phrase).unwrap();
        assert_eq!(mnemonic.phrase(), phrase);
    }

    #[test]
    fn test_derive_keypair() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::from_phrase(phrase).unwrap();
        
        let keypair1 = mnemonic.derive_ed25519_keypair(0, 0, 0, None).unwrap();
        let keypair2 = mnemonic.derive_ed25519_keypair(0, 0, 0, None).unwrap();
        
        // Same path should produce same keypair
        assert_eq!(keypair1.secret.as_bytes(), keypair2.secret.as_bytes());
        
        // Different index should produce different keypair
        let keypair3 = mnemonic.derive_ed25519_keypair(0, 0, 1, None).unwrap();
        assert_ne!(keypair1.secret.as_bytes(), keypair3.secret.as_bytes());
    }

    #[test]
    fn test_derivation_path() {
        let path = Mnemonic::default_derivation_path(0, 0, 0);
        assert_eq!(path, "m/44'/177017'/0'/0'/0'");
    }
}
