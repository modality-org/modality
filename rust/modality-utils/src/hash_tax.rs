use sha1::{Sha1, Digest};
use sha2::{Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::error::Error;
use num_bigint::BigUint;
use num_bigint::ToBigUint;
use num_traits::Num;
use randomx_rs::{RandomXFlag, RandomXVM};

const DEFAULT_MAX_TRIES: u128 = 100_000_000_000;
const DEFAULT_HASH_FUNC_NAME: &str = "randomx";
const DEFAULT_DIFFICULTY_COEFFICIENT: u128 = 0xffff;
const DEFAULT_DIFFICULTY_EXPONENT: u128 = 0x1d;
const DEFAULT_DIFFICULTY_BASE: u128 = 8;
const RANDOMX_KEY: &[u8] = b"modality-network-randomx-key";

lazy_static::lazy_static! {
    static ref HASH_FUNC_HEXADECIMAL_LENGTH: HashMap<&'static str, usize> = {
        let mut map = HashMap::new();
        map.insert("sha1", 40);
        map.insert("sha256", 64);
        map.insert("sha384", 96);
        map.insert("sha512", 128);
        map.insert("randomx", 64);  // RandomX outputs 256 bits = 64 hex chars
        map
    };
}

/// Create a RandomX VM instance using recommended flags
fn create_randomx_vm() -> Result<RandomXVM, Box<dyn Error>> {
    log::debug!("🔧 Initializing RandomX VM with recommended flags...");
    let flags = RandomXFlag::get_recommended_flags();
    log::debug!("🔧 Creating RandomX cache with key...");
    let cache = randomx_rs::RandomXCache::new(flags, RANDOMX_KEY)
        .map_err(|e| format!("Failed to create RandomX cache: {}", e))?;
    log::debug!("🔧 Initializing RandomX VM...");
    let vm = RandomXVM::new(flags, Some(cache), None)
        .map_err(|e| format!("Failed to initialize RandomX VM: {}", e))?;
    log::info!("✅ RandomX VM initialized successfully");
    Ok(vm)
}

/// Hash data using RandomX
fn hash_with_randomx(data: &str) -> Result<String, Box<dyn Error>> {
    let vm = create_randomx_vm()?;
    let hash_bytes = vm.calculate_hash(data.as_bytes())
        .map_err(|e| format!("RandomX hashing failed: {}", e))?;
    Ok(hex::encode(hash_bytes))
}

#[allow(dead_code)]
pub fn mine(
    data: &str,
    difficulty: u128,
    max_tries: Option<u128>,
    hash_func_name: Option<&str>,
) -> Result<u128, Box<dyn Error>> {
    let max_tries = max_tries.unwrap_or(DEFAULT_MAX_TRIES);
    let hash_func_name = hash_func_name.unwrap_or(DEFAULT_HASH_FUNC_NAME);

    log::info!("⛏️  Starting mining with {} algorithm (difficulty: {})", hash_func_name, difficulty);

    let mut nonce = 0;
    let mut try_count = 0;
    let mut last_status_log = std::time::Instant::now();
    let status_interval = std::time::Duration::from_secs(10);
    let mut last_try_count = 0;

    while try_count < max_tries {
        try_count += 1;
        
        // Log periodic status updates (only if we're doing a lot of attempts)
        if try_count > 1000 && last_status_log.elapsed() >= status_interval {
            let attempts_since_last = try_count - last_try_count;
            let hash_rate = attempts_since_last as f64 / last_status_log.elapsed().as_secs_f64();
            log::info!("⛏️  Mining status: tried {} nonces, hash rate: {:.2} H/s, current nonce: {}", 
                try_count, hash_rate, nonce);
            last_status_log = std::time::Instant::now();
            last_try_count = try_count;
        }
        
        let hash = hash_with_nonce(data, nonce, hash_func_name)?;
        if is_hash_acceptable(&hash, difficulty, hash_func_name) {
            log::info!("✅ Found valid nonce {} after {} attempts", nonce, try_count);
            return Ok(nonce);
        }
        nonce += 1;
    }

    Err("maxTries reached, no nonce found".into())
}

#[allow(dead_code)]
pub fn hash_with_nonce(data: &str, nonce: u128, hash_func_name: &str) -> Result<String, Box<dyn Error>> {
    let hash = match hash_func_name {
        "sha1" => {
            let mut hasher = Sha1::new();
            hasher.update(format!("{}{}", data, nonce));
            format!("{:x}", hasher.finalize())
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(format!("{}{}", data, nonce));
            format!("{:x}", hasher.finalize())
        }
        "sha384" => {
            let mut hasher = Sha384::new();
            hasher.update(format!("{}{}", data, nonce));
            format!("{:x}", hasher.finalize())
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(format!("{}{}", data, nonce));
            format!("{:x}", hasher.finalize())
        }
        "randomx" => {
            let input = format!("{}{}", data, nonce);
            hash_with_randomx(&input)?
        }
        _ => return Err(format!("Unsupported hash function: {}", hash_func_name).into()),
    };

    Ok(hash)
}

pub fn difficulty_to_target_hash(
    difficulty: u128,
    hash_func_name: &str,
    coefficient: u128,
    exponent: u128,
    base: u128,
) -> String {
    let _hex_length = HASH_FUNC_HEXADECIMAL_LENGTH[hash_func_name];
    let max_target = coefficient.to_biguint().unwrap() << (exponent * base);
    let target_bignum = max_target / difficulty;
    target_bignum.to_str_radix(16)
}

pub fn is_hash_acceptable(hash: &str, difficulty: u128, hash_func_name: &str) -> bool {
    let target_hash = difficulty_to_target_hash(difficulty, hash_func_name, DEFAULT_DIFFICULTY_COEFFICIENT, DEFAULT_DIFFICULTY_EXPONENT, DEFAULT_DIFFICULTY_BASE);
    let hash_big_int = BigUint::from_str_radix(hash, 16).unwrap();
    let target_big_int = BigUint::from_str_radix(&target_hash, 16).unwrap();
    hash_big_int < target_big_int
}

#[allow(dead_code)]
pub fn validate_nonce(data: &str, nonce: u128, difficulty: u128, hash_func_name: &str) -> Result<bool, Box<dyn Error>> {
    let hash = hash_with_nonce(data, nonce, hash_func_name)?;
    Ok(is_hash_acceptable(&hash, difficulty, hash_func_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let data = String::from("data");
        // Use very low difficulty for RandomX (no scaling)
        // RandomX is slower than SHA256, so we need more tries
        let nonce = mine(&data, 1, Some(10000), Some("randomx")).unwrap();
        assert!(nonce < 10000);
    }

    #[test]
    fn test_randomx_hash() {
        // Test that RandomX hashing works
        let hash = hash_with_nonce("test", 0, "randomx").unwrap();
        assert_eq!(hash.len(), 64); // RandomX produces 256-bit hash = 64 hex chars
    }

    #[test]
    fn test_sha256_still_works() {
        // Ensure SHA256 still works for backwards compatibility
        let data = String::from("data");
        let nonce = mine(&data, 500, None, Some("sha256")).unwrap();
        assert_eq!(nonce, 2401); // Known value for SHA256
    }
}