//! Merkle tree utilities for computing hash roots
//!
//! This module provides a simple binary Merkle tree implementation
//! for computing roots over a list of block hashes.

use sha2::{Sha256, Digest};

/// Compute the SHA-256 hash of two concatenated hashes
fn hash_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().to_vec()
}

/// Compute the Merkle root of a list of hex-encoded hashes
///
/// # Arguments
/// * `hashes` - List of hex-encoded hash strings
///
/// # Returns
/// * Hex-encoded Merkle root, or empty string if input is empty
///
/// # Algorithm
/// Uses a binary Merkle tree construction:
/// - If odd number of leaves, duplicate the last one
/// - Recursively pair and hash until single root remains
///
/// # Example
/// ```
/// use modal_common::merkle::compute_merkle_root;
///
/// let hashes = vec!["abc123", "def456", "789abc"];
/// let root = compute_merkle_root(&hashes);
/// assert!(!root.is_empty());
/// ```
pub fn compute_merkle_root(hashes: &[&str]) -> String {
    if hashes.is_empty() {
        return String::new();
    }
    
    if hashes.len() == 1 {
        return hashes[0].to_string();
    }
    
    // Convert hex strings to bytes
    let mut level: Vec<Vec<u8>> = hashes
        .iter()
        .map(|h| {
            // Handle both hex-encoded and raw string hashes
            hex::decode(h).unwrap_or_else(|_| {
                // If not valid hex, hash the string directly
                let mut hasher = Sha256::new();
                hasher.update(h.as_bytes());
                hasher.finalize().to_vec()
            })
        })
        .collect();
    
    // Build tree bottom-up
    while level.len() > 1 {
        let mut next_level = Vec::new();
        
        // Process pairs
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            // If odd, duplicate the last element
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            
            next_level.push(hash_pair(left, right));
            i += 2;
        }
        
        level = next_level;
    }
    
    hex::encode(&level[0])
}

/// Compute Merkle root from a list of owned strings
pub fn compute_merkle_root_owned(hashes: &[String]) -> String {
    let refs: Vec<&str> = hashes.iter().map(|s| s.as_str()).collect();
    compute_merkle_root(&refs)
}

/// Verify that a hash is included in a Merkle tree given a proof
///
/// # Arguments
/// * `hash` - The hex-encoded hash to verify
/// * `root` - The expected Merkle root (hex-encoded)
/// * `proof` - List of (hash, is_left) pairs forming the proof path
///
/// # Returns
/// * true if the proof is valid, false otherwise
pub fn verify_merkle_proof(
    hash: &str,
    root: &str,
    proof: &[(String, bool)],
) -> bool {
    let mut current = hex::decode(hash).unwrap_or_else(|_| {
        let mut hasher = Sha256::new();
        hasher.update(hash.as_bytes());
        hasher.finalize().to_vec()
    });
    
    for (sibling_hex, is_left) in proof {
        let sibling = match hex::decode(sibling_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        current = if *is_left {
            hash_pair(&sibling, &current)
        } else {
            hash_pair(&current, &sibling)
        };
    }
    
    hex::encode(&current) == root
}

/// Generate a Merkle proof for a hash at a given index
///
/// # Arguments
/// * `hashes` - List of all hashes in the tree
/// * `index` - Index of the hash to generate proof for
///
/// # Returns
/// * Some(proof) where proof is a list of (sibling_hash, is_left) pairs
/// * None if index is out of bounds
pub fn generate_merkle_proof(hashes: &[&str], index: usize) -> Option<Vec<(String, bool)>> {
    if index >= hashes.len() || hashes.is_empty() {
        return None;
    }
    
    if hashes.len() == 1 {
        return Some(vec![]);
    }
    
    // Convert hex strings to bytes
    let mut level: Vec<Vec<u8>> = hashes
        .iter()
        .map(|h| {
            hex::decode(h).unwrap_or_else(|_| {
                let mut hasher = Sha256::new();
                hasher.update(h.as_bytes());
                hasher.finalize().to_vec()
            })
        })
        .collect();
    
    let mut proof = Vec::new();
    let mut current_index = index;
    
    // Build proof bottom-up
    while level.len() > 1 {
        let mut next_level = Vec::new();
        
        // Find sibling and add to proof
        let sibling_index = if current_index % 2 == 0 {
            // Current is left, sibling is right
            if current_index + 1 < level.len() {
                current_index + 1
            } else {
                current_index // Duplicate case
            }
        } else {
            // Current is right, sibling is left
            current_index - 1
        };
        
        let is_left = sibling_index < current_index;
        proof.push((hex::encode(&level[sibling_index]), is_left));
        
        // Build next level
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            next_level.push(hash_pair(left, right));
            i += 2;
        }
        
        level = next_level;
        current_index /= 2;
    }
    
    Some(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_merkle_root() {
        let hashes: Vec<&str> = vec![];
        assert_eq!(compute_merkle_root(&hashes), "");
    }

    #[test]
    fn test_single_hash() {
        let hashes = vec!["abc123"];
        assert_eq!(compute_merkle_root(&hashes), "abc123");
    }

    #[test]
    fn test_two_hashes() {
        let hashes = vec!["abc123", "def456"];
        let root = compute_merkle_root(&hashes);
        assert!(!root.is_empty());
        assert_ne!(root, "abc123");
        assert_ne!(root, "def456");
    }

    #[test]
    fn test_odd_number_of_hashes() {
        let hashes = vec!["hash1", "hash2", "hash3"];
        let root = compute_merkle_root(&hashes);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_deterministic() {
        let hashes = vec!["a", "b", "c", "d"];
        let root1 = compute_merkle_root(&hashes);
        let root2 = compute_merkle_root(&hashes);
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_order_matters() {
        let hashes1 = vec!["a", "b"];
        let hashes2 = vec!["b", "a"];
        assert_ne!(compute_merkle_root(&hashes1), compute_merkle_root(&hashes2));
    }

    #[test]
    fn test_owned_version() {
        let hashes = vec!["abc".to_string(), "def".to_string()];
        let root = compute_merkle_root_owned(&hashes);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_valid_hex_hashes() {
        // Valid hex hashes (like block hashes)
        let hashes = vec![
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
            "f1e2d3c4b5a6f1e2d3c4b5a6f1e2d3c4b5a6f1e2d3c4b5a6f1e2d3c4b5a6f1e2",
        ];
        let root = compute_merkle_root(&hashes);
        assert_eq!(root.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let hashes = vec!["a", "b", "c", "d"];
        let root = compute_merkle_root(&hashes);
        
        // Generate and verify proof for each hash
        for i in 0..hashes.len() {
            let proof = generate_merkle_proof(&hashes, i).unwrap();
            assert!(verify_merkle_proof(hashes[i], &root, &proof));
        }
    }

    #[test]
    fn test_invalid_proof() {
        let hashes = vec!["a", "b", "c", "d"];
        let root = compute_merkle_root(&hashes);
        
        // Proof for "a" should not verify "b"
        let proof = generate_merkle_proof(&hashes, 0).unwrap();
        assert!(!verify_merkle_proof("b", &root, &proof));
    }

    #[test]
    fn test_proof_out_of_bounds() {
        let hashes = vec!["a", "b"];
        assert!(generate_merkle_proof(&hashes, 5).is_none());
    }

    #[test]
    fn test_single_element_proof() {
        let hashes = vec!["a"];
        let root = compute_merkle_root(&hashes);
        let proof = generate_merkle_proof(&hashes, 0).unwrap();
        assert!(proof.is_empty());
        // For single element, root equals the element
        assert_eq!(root, "a");
    }
}

