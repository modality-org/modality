use sha2::{Sha256, Digest};

/// Fisher-Yates shuffle algorithm that produces a deterministic shuffled array
/// of integers from 0 to size-1 based on a seed.
///
/// # Arguments
/// * `seed` - A seed value used to generate deterministic randomness
/// * `size` - The size of the array to shuffle (will contain values 0..size-1)
///
/// # Returns
/// A Vec<usize> containing the shuffled integers from 0 to size-1
///
/// # Example
/// ```
/// use modal_common::shuffle::fisher_yates_shuffle;
///
/// let shuffled = fisher_yates_shuffle(12345, 10);
/// assert_eq!(shuffled.len(), 10);
/// // With the same seed, we always get the same shuffle
/// let shuffled2 = fisher_yates_shuffle(12345, 10);
/// assert_eq!(shuffled, shuffled2);
/// ```
pub fn fisher_yates_shuffle(seed: u64, size: usize) -> Vec<usize> {
    if size == 0 {
        return Vec::new();
    }

    // Initialize array with values 0 to size-1
    let mut array: Vec<usize> = (0..size).collect();

    // Use seed to create a deterministic pseudo-random number generator
    let mut rng_state = seed;

    // Fisher-Yates shuffle algorithm
    for i in (1..size).rev() {
        // Generate a deterministic random index between 0 and i (inclusive)
        let j = deterministic_random(&mut rng_state, i + 1);
        
        // Swap elements at positions i and j
        array.swap(i, j);
    }

    array
}

/// Generates a deterministic random number in the range [0, max) using SHA256
fn deterministic_random(state: &mut u64, max: usize) -> usize {
    // Hash the current state to get pseudo-random bytes
    let mut hasher = Sha256::new();
    hasher.update(state.to_le_bytes());
    let hash = hasher.finalize();
    
    // Update state for next call
    *state = u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3],
        hash[4], hash[5], hash[6], hash[7],
    ]);
    
    // Convert hash to a number in range [0, max)
    let value = u64::from_le_bytes([
        hash[8], hash[9], hash[10], hash[11],
        hash[12], hash[13], hash[14], hash[15],
    ]);
    
    (value % max as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_fisher_yates_shuffle_basic() {
        let size = 10;
        let shuffled = fisher_yates_shuffle(12345, size);
        
        assert_eq!(shuffled.len(), size);
        
        // Verify all numbers from 0 to size-1 are present
        let set: HashSet<usize> = shuffled.iter().copied().collect();
        assert_eq!(set.len(), size);
        for i in 0..size {
            assert!(set.contains(&i));
        }
    }

    #[test]
    fn test_fisher_yates_shuffle_deterministic() {
        let seed = 42;
        let size = 100;
        
        let shuffled1 = fisher_yates_shuffle(seed, size);
        let shuffled2 = fisher_yates_shuffle(seed, size);
        
        // Same seed should produce same shuffle
        assert_eq!(shuffled1, shuffled2);
    }

    #[test]
    fn test_fisher_yates_shuffle_different_seeds() {
        let size = 50;
        
        let shuffled1 = fisher_yates_shuffle(111, size);
        let shuffled2 = fisher_yates_shuffle(222, size);
        
        // Different seeds should produce different shuffles (with very high probability)
        assert_ne!(shuffled1, shuffled2);
    }

    #[test]
    fn test_fisher_yates_shuffle_empty() {
        let shuffled = fisher_yates_shuffle(123, 0);
        assert_eq!(shuffled.len(), 0);
    }

    #[test]
    fn test_fisher_yates_shuffle_single() {
        let shuffled = fisher_yates_shuffle(123, 1);
        assert_eq!(shuffled, vec![0]);
    }

    #[test]
    fn test_fisher_yates_shuffle_small() {
        let size = 5;
        let shuffled = fisher_yates_shuffle(999, size);
        
        assert_eq!(shuffled.len(), size);
        
        // Verify it's a valid permutation
        let set: HashSet<usize> = shuffled.iter().copied().collect();
        assert_eq!(set.len(), size);
        
        // Verify all expected values are present
        for i in 0..size {
            assert!(set.contains(&i));
        }
    }

    #[test]
    fn test_fisher_yates_shuffle_large() {
        let size = 1000;
        let shuffled = fisher_yates_shuffle(777, size);
        
        assert_eq!(shuffled.len(), size);
        
        // Verify all numbers are present
        let set: HashSet<usize> = shuffled.iter().copied().collect();
        assert_eq!(set.len(), size);
        for i in 0..size {
            assert!(set.contains(&i));
        }
    }

    #[test]
    fn test_deterministic_random_distribution() {
        // Test that our random number generator produces reasonable distribution
        let mut state = 12345u64;
        let max = 10;
        let iterations = 1000;
        
        let mut counts = vec![0; max];
        for _ in 0..iterations {
            let value = deterministic_random(&mut state, max);
            assert!(value < max);
            counts[value] += 1;
        }
        
        // Each value should appear at least once in 1000 iterations (with very high probability)
        for count in counts {
            assert!(count > 0);
        }
    }

    #[test]
    fn test_different_sizes_same_seed() {
        let seed = 42;
        
        let shuffled5 = fisher_yates_shuffle(seed, 5);
        let shuffled10 = fisher_yates_shuffle(seed, 10);
        
        assert_eq!(shuffled5.len(), 5);
        assert_eq!(shuffled10.len(), 10);
        
        // Verify both are valid permutations
        let set5: HashSet<usize> = shuffled5.iter().copied().collect();
        let set10: HashSet<usize> = shuffled10.iter().copied().collect();
        
        assert_eq!(set5.len(), 5);
        assert_eq!(set10.len(), 10);
    }
}

