use sha1::{Sha1, Digest};
use sha2::{Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::error::Error;
use num_bigint::BigUint;
use num_bigint::ToBigUint;
use num_traits::Num;

const DEFAULT_MAX_TRIES: u128 = 100_000_000_000;
const DEFAULT_HASH_FUNC_NAME: &str = "sha256";
const DEFAULT_DIFFICULTY_COEFFICIENT: u128 = 0xffff;
const DEFAULT_DIFFICULTY_EXPONENT: u128 = 0x1d;
const DEFAULT_DIFFICULTY_BASE: u128 = 8;

lazy_static::lazy_static! {
    static ref HASH_FUNC_HEXADECIMAL_LENGTH: HashMap<&'static str, usize> = {
        let mut map = HashMap::new();
        map.insert("sha1", 40);
        map.insert("sha256", 64);
        map.insert("sha384", 96);
        map.insert("sha512", 128);
        map
    };
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

    let mut nonce = 0;
    let mut try_count = 0;

    while try_count < max_tries {
        try_count += 1;
        let hash = hash_with_nonce(data, nonce, hash_func_name)?;
        if is_hash_acceptable(&hash, difficulty, hash_func_name) {
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
        let nonce = mine(&data, 500, None, None).unwrap();
        assert_eq!(nonce, 2401);
    }
}