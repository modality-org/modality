use crate::NetworkDatastore;
use crate::models::{MinerBlock, SequencerSet};
use anyhow::Result;

/// Generate a sequencer set from a completed mining epoch
/// 
/// This function:
/// 1. Gets all blocks from the epoch
/// 2. Shuffles the nominations using XOR of nonces
/// 3. Selects top 27 as nominated sequencers
/// 4. Selects top 13 from staking (placeholder for now)
/// 5. Selects bottom 13 from nominations as alternates
pub async fn generate_sequencer_set_from_epoch(
    datastore: &NetworkDatastore,
    epoch: u64,
) -> Result<SequencerSet> {
    // Get all canonical blocks from this epoch
    let epoch_blocks = MinerBlock::find_canonical_by_epoch(datastore, epoch).await?;
    
    if epoch_blocks.is_empty() {
        anyhow::bail!("No blocks found for epoch {}", epoch);
    }

    // Calculate seed from XOR of all nonces
    let seed = calculate_epoch_seed(&epoch_blocks);
    
    // Get all nominated peer IDs
    let peer_ids: Vec<String> = epoch_blocks
        .iter()
        .map(|b| b.nominated_peer_id.clone())
        .collect();
    
    // Shuffle using Fisher-Yates with the seed
    let shuffled_peer_ids = shuffle_peer_ids(seed, &peer_ids);
    
    // Select sequencers
    let nominated_sequencers = shuffled_peer_ids
        .iter()
        .take(27)
        .cloned()
        .collect::<Vec<String>>();
    
    // For alternates, take from the bottom 13 of the shuffled list
    let total_peers = shuffled_peer_ids.len();
    let alternate_sequencers = if total_peers > 27 {
        shuffled_peer_ids
            .iter()
            .skip(total_peers.saturating_sub(13))
            .take(13)
            .cloned()
            .collect::<Vec<String>>()
    } else {
        Vec::new()
    };
    
    // TODO: Implement actual staking mechanism
    // For now, staked sequencers is empty
    let staked_sequencers = Vec::new();
    
    // Create and return the sequencer set
    Ok(SequencerSet::new(
        epoch,
        epoch + 1, // This set will be used for the next mining epoch
        nominated_sequencers,
        staked_sequencers,
        alternate_sequencers,
    ))
}

/// Calculate seed from XOR of all block nonces
fn calculate_epoch_seed(blocks: &[MinerBlock]) -> u64 {
    let mut seed = 0u64;
    for block in blocks {
        // Parse nonce and XOR with seed
        if let Ok(nonce) = block.nonce.parse::<u128>() {
            seed ^= nonce as u64;
        }
    }
    seed
}

/// Shuffle peer IDs using Fisher-Yates algorithm with a seed
fn shuffle_peer_ids(seed: u64, peer_ids: &[String]) -> Vec<String> {
    let indices = modality_utils::shuffle::fisher_yates_shuffle(seed, peer_ids.len());
    indices.into_iter().map(|i| peer_ids[i].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_epoch_seed() {
        let blocks = vec![
            MinerBlock::new_canonical(
                "hash1".to_string(),
                0,
                0,
                0,
                "0".to_string(),
                "data".to_string(),
                100,
                1000,
                "peer1".to_string(),
                42,
            ),
            MinerBlock::new_canonical(
                "hash2".to_string(),
                1,
                0,
                1,
                "hash1".to_string(),
                "data".to_string(),
                200,
                1000,
                "peer2".to_string(),
                43,
            ),
        ];
        
        let seed = calculate_epoch_seed(&blocks);
        assert_eq!(seed, 100 ^ 200); // XOR of the two nonces
    }

    #[test]
    fn test_shuffle_peer_ids() {
        let peer_ids = vec![
            "peer1".to_string(),
            "peer2".to_string(),
            "peer3".to_string(),
        ];
        
        let shuffled1 = shuffle_peer_ids(42, &peer_ids);
        let shuffled2 = shuffle_peer_ids(42, &peer_ids);
        
        // Same seed should produce same result
        assert_eq!(shuffled1, shuffled2);
        
        // Should contain all peers
        assert_eq!(shuffled1.len(), 3);
        for peer in &peer_ids {
            assert!(shuffled1.contains(peer));
        }
        
        // Different seed should produce different result (with high probability)
        let shuffled3 = shuffle_peer_ids(999, &peer_ids);
        assert_ne!(shuffled1, shuffled3);
    }
}


