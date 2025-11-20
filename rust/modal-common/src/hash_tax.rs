use sha1::{Sha1, Digest};
use sha2::{Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::error::Error;
use num_bigint::BigUint;
use num_bigint::ToBigUint;
use num_traits::Num;
use randomx_rs::{RandomXFlag, RandomXVM};
use serde::{Deserialize};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DEFAULT_MAX_TRIES: u128 = 100_000_000_000;
const DEFAULT_HASH_FUNC_NAME: &str = "randomx";
const DEFAULT_DIFFICULTY_COEFFICIENT: u128 = 0xffff;
const DEFAULT_DIFFICULTY_EXPONENT: u128 = 0x1d;
const DEFAULT_DIFFICULTY_BASE: u128 = 8;
const RANDOMX_KEY: &[u8] = b"modality-network-randomx-key";

/// RandomX-specific hashing parameters
#[derive(Debug, Clone, Deserialize)]
pub struct RandomXParams {
    pub key: Option<String>,  // Custom key (default: "modality-network-randomx-key")
    pub flags: Option<String>, // "recommended", "light", "full", or comma-separated flags
}

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
    
    /// Global flag to signal mining should stop (e.g., on Ctrl-C)
    static ref MINING_SHOULD_STOP: Arc<AtomicBool> = {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        
        // Set up signal handler for graceful shutdown
        #[cfg(unix)]
        {
            use std::sync::Mutex;
            static HANDLER_INSTALLED: Mutex<bool> = Mutex::new(false);
            
            let mut installed = HANDLER_INSTALLED.lock().unwrap();
            if !*installed {
                if let Err(e) = ctrlc::set_handler(move || {
                    log::info!("🛑 Received shutdown signal (Ctrl-C), stopping mining...");
                    flag_clone.store(true, Ordering::Relaxed);
                }) {
                    log::warn!("Failed to set Ctrl-C handler: {}", e);
                }
                *installed = true;
            }
        }
        
        flag
    };
}

thread_local! {
    /// Thread-local RandomX VM instance that is initialized once per thread and reused
    /// This avoids the expensive initialization cost (2-3 seconds) for every hash
    static RANDOMX_VM: RefCell<Option<RandomXVM>> = RefCell::new(None);
    
    /// Thread-local RandomX parameters for custom configuration
    static RANDOMX_PARAMS: RefCell<Option<RandomXParams>> = RefCell::new(None);
}

/// Parse RandomX flags from string
fn parse_randomx_flags(flags_str: &str) -> RandomXFlag {
    // For now, always use recommended flags
    // The randomx-rs crate may not expose individual flag constants
    log::debug!("Using RandomX recommended flags (custom flags not yet supported: {})", flags_str);
    RandomXFlag::get_recommended_flags()
}

/// Set RandomX parameters for the current thread
pub fn set_randomx_params(params: Option<RandomXParams>) {
    RANDOMX_PARAMS.with(|p| {
        *p.borrow_mut() = params;
    });
    // Clear the VM so it reinitializes with new params
    RANDOMX_VM.with(|vm| {
        *vm.borrow_mut() = None;
    });
}

/// Set RandomX parameters from JSON value
pub fn set_randomx_params_from_json(params_json: Option<&serde_json::Value>) {
    let params = params_json.and_then(|v| serde_json::from_value::<RandomXParams>(v.clone()).ok());
    set_randomx_params(params);
}


/// Get or create the thread-local RandomX VM instance
fn with_randomx_vm<F, R>(f: F) -> Result<R, Box<dyn Error>>
where
    F: FnOnce(&RandomXVM) -> Result<R, Box<dyn Error>>,
{
    RANDOMX_VM.with(|vm_cell| {
        let mut vm_opt = vm_cell.borrow_mut();
        
        if vm_opt.is_none() {
            log::info!("🔧 Initializing RandomX VM (one-time setup)...");
            
            // Get custom params if available
            let params = RANDOMX_PARAMS.with(|p| p.borrow().clone());
            
            // Determine flags
            let flags = if let Some(ref p) = params {
                if let Some(ref flags_str) = p.flags {
                    log::debug!("Using custom RandomX flags: {}", flags_str);
                    parse_randomx_flags(flags_str)
                } else {
                    RandomXFlag::get_recommended_flags()
                }
            } else {
                RandomXFlag::get_recommended_flags()
            };
            
            // Determine key
            let key = if let Some(ref p) = params {
                if let Some(ref custom_key) = p.key {
                    log::debug!("Using custom RandomX key");
                    custom_key.as_bytes()
                } else {
                    RANDOMX_KEY
                }
            } else {
                RANDOMX_KEY
            };
            
            log::debug!("🔧 Creating RandomX cache with key...");
            let cache = randomx_rs::RandomXCache::new(flags, key)
                .map_err(|e| format!("Failed to create RandomX cache: {}", e))?;
            log::debug!("🔧 Creating RandomX VM...");
            let vm = RandomXVM::new(flags, Some(cache), None)
                .map_err(|e| format!("Failed to initialize RandomX VM: {}", e))?;
            log::info!("✅ RandomX VM initialized successfully (ready for mining)");
            *vm_opt = Some(vm);
        }
        
        let vm = vm_opt.as_ref().unwrap();
        f(vm)
    })
}

/// Hash data using RandomX (uses thread-local VM for efficiency)
fn hash_with_randomx(data: &str) -> Result<String, Box<dyn Error>> {
    with_randomx_vm(|vm| {
        let hash_bytes = vm.calculate_hash(data.as_bytes())
            .map_err(|e| format!("RandomX hashing failed: {}", e))?;
        Ok(hex::encode(hash_bytes))
    })
}

/// Mining result including nonce and stats
#[derive(Debug, Clone)]
pub struct MiningResult {
    pub nonce: u128,
    pub attempts: u128,
    pub duration_secs: f64,
}

impl MiningResult {
    pub fn hashrate(&self) -> f64 {
        if self.duration_secs > 0.0 {
            self.attempts as f64 / self.duration_secs
        } else {
            0.0
        }
    }
}

#[allow(dead_code)]
pub fn mine(
    data: &str,
    difficulty: u128,
    max_tries: Option<u128>,
    hash_func_name: Option<&str>,
) -> Result<u128, Box<dyn Error>> {
    mine_with_stats(data, difficulty, max_tries, hash_func_name, None)
        .map(|result| result.nonce)
}

/// Mine with detailed statistics
#[allow(dead_code)]
pub fn mine_with_stats(
    data: &str,
    difficulty: u128,
    max_tries: Option<u128>,
    hash_func_name: Option<&str>,
    mining_delay_ms: Option<u64>,
) -> Result<MiningResult, Box<dyn Error>> {
    let max_tries = max_tries.unwrap_or(DEFAULT_MAX_TRIES);
    let hash_func_name = hash_func_name.unwrap_or(DEFAULT_HASH_FUNC_NAME);
    let mining_delay = mining_delay_ms.unwrap_or(0);

    log::info!("⛏️  Starting mining with {} algorithm (difficulty: {})", hash_func_name, difficulty);
    
    if mining_delay > 0 {
        log::info!("🐌 Mining slowdown enabled: {}ms delay per attempt (for testing)", mining_delay);
    }

    let start_time = std::time::Instant::now();
    let mut nonce = 0;
    let mut try_count = 0;
    let mut last_status_log = std::time::Instant::now();
    let status_interval = std::time::Duration::from_secs(10);
    let mut last_try_count = 0;

    while try_count < max_tries {
        // Check if we should stop mining (e.g., Ctrl-C was pressed)
        if MINING_SHOULD_STOP.load(Ordering::Relaxed) {
            log::info!("🛑 Mining stopped by shutdown signal after {} attempts", try_count);
            return Err("Mining interrupted by shutdown signal".into());
        }
        
        try_count += 1;
        
        // Add artificial delay for testing race conditions
        if mining_delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(mining_delay));
        }
        
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
            let duration = start_time.elapsed();
            log::info!("✅ Found valid nonce {} after {} attempts", nonce, try_count);
            return Ok(MiningResult {
                nonce,
                attempts: try_count,
                duration_secs: duration.as_secs_f64(),
            });
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