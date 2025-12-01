//! Fork choice rules for chain selection.
//!
//! This module implements the canonical fork choice rules used throughout
//! the node for determining which chain to follow when forks occur.
//!
//! The fork choice rules in priority order are:
//! 1. Higher cumulative difficulty wins
//! 2. Longer chain wins (if difficulty is equal)
//! 3. Lower block hash wins (as final tiebreaker)

use modal_datastore::models::MinerBlock;

/// Result of comparing two chains or blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkChoiceResult {
    /// The first/local chain wins
    KeepLocal,
    /// The second/remote chain wins
    AdoptRemote,
    /// Chains are identical
    Equal,
}

/// Detailed comparison information between two chains
#[derive(Debug, Clone)]
pub struct ChainComparison {
    /// The fork choice result
    pub result: ForkChoiceResult,
    /// Local chain cumulative difficulty
    pub local_difficulty: u128,
    /// Remote chain cumulative difficulty
    pub remote_difficulty: u128,
    /// Local chain length
    pub local_length: u64,
    /// Remote chain length
    pub remote_length: u64,
    /// Human-readable reason for the decision
    pub reason: String,
}

/// Compare two chains and determine which one should be canonical.
///
/// Uses fork choice rules in priority order:
/// 1. Higher cumulative difficulty wins
/// 2. Longer chain wins (if difficulty is equal)
///
/// # Arguments
/// * `local_difficulty` - Cumulative difficulty of the local chain
/// * `local_length` - Number of blocks in the local chain
/// * `remote_difficulty` - Cumulative difficulty of the remote chain
/// * `remote_length` - Number of blocks in the remote chain
///
/// # Returns
/// A `ChainComparison` with the result and reasoning
pub fn compare_chains(
    local_difficulty: u128,
    local_length: u64,
    remote_difficulty: u128,
    remote_length: u64,
) -> ChainComparison {
    let (result, reason) = if remote_difficulty > local_difficulty {
        (
            ForkChoiceResult::AdoptRemote,
            format!(
                "Remote chain has higher cumulative difficulty ({} > {})",
                remote_difficulty, local_difficulty
            ),
        )
    } else if remote_difficulty < local_difficulty {
        (
            ForkChoiceResult::KeepLocal,
            format!(
                "Local chain has higher cumulative difficulty ({} > {})",
                local_difficulty, remote_difficulty
            ),
        )
    } else if remote_length > local_length {
        // Equal difficulty - use length as tiebreaker
        (
            ForkChoiceResult::AdoptRemote,
            format!(
                "Equal difficulty, remote chain is longer ({} > {} blocks)",
                remote_length, local_length
            ),
        )
    } else if remote_length < local_length {
        (
            ForkChoiceResult::KeepLocal,
            format!(
                "Equal difficulty, local chain is longer ({} > {} blocks)",
                local_length, remote_length
            ),
        )
    } else {
        (
            ForkChoiceResult::Equal,
            "Chains have equal difficulty and length".to_string(),
        )
    };

    ChainComparison {
        result,
        local_difficulty,
        remote_difficulty,
        local_length,
        remote_length,
        reason,
    }
}

/// Compare two blocks at the same height for fork choice.
///
/// Uses fork choice rules:
/// 1. Higher difficulty wins
/// 2. Earlier seen_at timestamp wins (first-seen rule)
/// 3. Lower hash wins (final tiebreaker)
///
/// # Arguments
/// * `new_block` - The new/incoming block
/// * `existing_block` - The existing canonical block
///
/// # Returns
/// `true` if the new block should replace the existing block
pub fn should_replace_block(new_block: &MinerBlock, existing_block: &MinerBlock) -> bool {
    let new_difficulty = new_block.get_actualized_difficulty_u128().unwrap_or(0);
    let existing_difficulty = existing_block.get_actualized_difficulty_u128().unwrap_or(0);

    if new_difficulty > existing_difficulty {
        // Rule 1: Higher actualized difficulty wins
        log::info!(
            "Fork choice: new block has higher actualized difficulty ({} > {})",
            new_difficulty,
            existing_difficulty
        );
        return true;
    }
    
    if new_difficulty < existing_difficulty {
        return false;
    }

    // Equal difficulty - check first-seen (seen_at timestamp)
    match (&new_block.seen_at, &existing_block.seen_at) {
        (Some(new_seen), Some(existing_seen)) => {
            if new_seen < existing_seen {
                log::info!(
                    "Fork choice: equal difficulty, new block seen earlier ({} < {})",
                    new_seen,
                    existing_seen
                );
                return true;
            }
            if new_seen > existing_seen {
                return false;
            }
            // Same seen_at - use hash as tiebreaker
            if new_block.hash < existing_block.hash {
                log::info!("Fork choice: equal difficulty and time, new block has lower hash");
                return true;
            }
            false
        }
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => {
            // No timestamps - use hash as tiebreaker
            if new_block.hash < existing_block.hash {
                log::info!(
                    "Fork choice: equal difficulty, no timestamps, new block has lower hash"
                );
                return true;
            }
            false
        }
    }
}

/// Detailed fork choice result for block comparison
#[derive(Debug, Clone)]
pub struct BlockForkChoiceResult {
    /// Whether to replace the existing block
    pub should_replace: bool,
    /// New block difficulty
    pub new_difficulty: u128,
    /// Existing block difficulty
    pub existing_difficulty: u128,
    /// Human-readable reason
    pub reason: String,
}

/// Compare two blocks and return detailed fork choice result.
///
/// # Arguments
/// * `new_block` - The new/incoming block
/// * `existing_block` - The existing canonical block
///
/// # Returns
/// Detailed fork choice result with reasoning
pub fn compare_blocks(new_block: &MinerBlock, existing_block: &MinerBlock) -> BlockForkChoiceResult {
    let new_difficulty = new_block.get_actualized_difficulty_u128().unwrap_or(0);
    let existing_difficulty = existing_block.get_actualized_difficulty_u128().unwrap_or(0);
    
    let (should_replace, reason) = if new_difficulty > existing_difficulty {
        (true, format!("Higher actualized difficulty ({} > {})", new_difficulty, existing_difficulty))
    } else if new_difficulty < existing_difficulty {
        (false, format!("Lower actualized difficulty ({} < {})", new_difficulty, existing_difficulty))
    } else {
        // Equal difficulty - check timestamps then hash
        match (&new_block.seen_at, &existing_block.seen_at) {
            (Some(new_seen), Some(existing_seen)) if new_seen < existing_seen => {
                (true, format!("Equal difficulty, seen earlier ({} < {})", new_seen, existing_seen))
            }
            (Some(new_seen), Some(existing_seen)) if new_seen > existing_seen => {
                (false, format!("Equal difficulty, seen later ({} > {})", new_seen, existing_seen))
            }
            (Some(_), None) => (true, "Equal difficulty, has timestamp vs none".to_string()),
            (None, Some(_)) => (false, "Equal difficulty, no timestamp vs has one".to_string()),
            _ => {
                // Same timestamps or both none - use hash
                if new_block.hash < existing_block.hash {
                    (true, format!("Equal difficulty, lower hash ({} < {})", &new_block.hash[..16], &existing_block.hash[..16]))
                } else {
                    (false, format!("Equal difficulty, higher/equal hash ({} >= {})", &new_block.hash[..16], &existing_block.hash[..16]))
                }
            }
        }
    };
    
    BlockForkChoiceResult {
        should_replace,
        new_difficulty,
        existing_difficulty,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_chains_difficulty_wins() {
        let result = compare_chains(100, 10, 200, 5);
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);
        
        let result = compare_chains(200, 5, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn test_compare_chains_length_tiebreaker() {
        let result = compare_chains(100, 10, 100, 15);
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);
        
        let result = compare_chains(100, 15, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn test_compare_chains_equal() {
        let result = compare_chains(100, 10, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::Equal);
    }
}

