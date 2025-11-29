use crate::DatastoreManager;
use crate::models::{miner::MinerBlock, validator::ValidatorSet};
use anyhow::Result;

/// Get validator set for an epoch (multi-store version)
pub async fn get_validator_set_for_epoch_multi(
    datastore: &DatastoreManager,
    epoch: u64,
) -> Result<ValidatorSet> {
    // Check if static validators are configured
    if let Some(static_validators) = datastore.get_static_validators().await? {
        // Create validator set from static validators
        return Ok(ValidatorSet::new(
            epoch,
            epoch + 1, // This set will be used for the next mining epoch
            static_validators, // All validators are "nominated"
            Vec::new(), // No staked validators
            Vec::new(), // No alternate validators
        ));
    }
    
    // Fall back to dynamic validator selection from mining epochs
    generate_validator_set_from_epoch_multi(datastore, epoch).await
}

/// Get validator set for hybrid consensus (multi-store version)
/// 
/// In hybrid consensus, validators for mining epoch N are selected from
/// nominations in mining epoch N-2. This provides a 2-epoch lookback
/// to ensure the validator set is stable before being activated.
pub async fn get_validator_set_for_mining_epoch_hybrid_multi(
    mgr: &DatastoreManager,
    current_mining_epoch: u64,
) -> Result<Option<ValidatorSet>> {
    if current_mining_epoch < 2 {
        log::info!(
            "Mining epoch {} is too early for hybrid consensus (need >= 2)",
            current_mining_epoch
        );
        return Ok(None);
    }
    
    let nomination_epoch = current_mining_epoch - 2;
    
    log::info!(
        "Getting validator set for mining epoch {} from nominations in epoch {}",
        current_mining_epoch,
        nomination_epoch
    );
    
    match generate_validator_set_from_epoch_multi(mgr, nomination_epoch).await {
        Ok(mut validator_set) => {
            validator_set.mining_epoch = current_mining_epoch;
            Ok(Some(validator_set))
        }
        Err(e) => {
            log::error!(
                "Failed to generate validator set from epoch {}: {}",
                nomination_epoch,
                e
            );
            Err(e)
        }
    }
}

/// Generate a validator set from a completed mining epoch (multi-store version)
pub async fn generate_validator_set_from_epoch_multi(
    mgr: &DatastoreManager,
    epoch: u64,
) -> Result<ValidatorSet> {
    let all_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    let epoch_blocks: Vec<_> = all_blocks.into_iter().filter(|b| b.epoch == epoch).collect();
    
    if epoch_blocks.is_empty() {
        anyhow::bail!("No blocks found for epoch {}", epoch);
    }

    let seed = calculate_epoch_seed(&epoch_blocks);
    let peer_ids: Vec<String> = epoch_blocks.iter().map(|b| b.nominated_peer_id.clone()).collect();
    let shuffled_peer_ids = shuffle_peer_ids(seed, &peer_ids);
    
    let nominated_validators = shuffled_peer_ids.iter().take(27).cloned().collect();
    let total_peers = shuffled_peer_ids.len();
    let alternate_validators = if total_peers > 27 {
        shuffled_peer_ids.iter().skip(total_peers.saturating_sub(13)).take(13).cloned().collect()
    } else {
        Vec::new()
    };
    
    Ok(ValidatorSet::new(epoch, epoch + 1, nominated_validators, Vec::new(), alternate_validators))
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
    let indices = modal_common::shuffle::fisher_yates_shuffle(seed, peer_ids.len());
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

    #[tokio::test]
    async fn test_get_validator_set_for_epoch_with_static_validators() {
        // Create a datastore with static validators
        let datastore = DatastoreManager::create_in_memory().unwrap();
        let static_validators = vec![
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string(),
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string(),
            "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string(),
        ];
        datastore.set_static_validators(&static_validators).await.unwrap();
        
        // Get validator set for epoch 0
        let validator_set = get_validator_set_for_epoch_multi(&datastore, 0).await.unwrap();
        
        // Verify it uses the static validators
        assert_eq!(validator_set.epoch, 0);
        assert_eq!(validator_set.mining_epoch, 1);
        assert_eq!(validator_set.nominated_validators.len(), 3);
        assert_eq!(validator_set.staked_validators.len(), 0);
        assert_eq!(validator_set.alternate_validators.len(), 0);
        
        // Verify all static validators are present
        for validator in &static_validators {
            assert!(validator_set.nominated_validators.contains(validator));
        }
    }

    #[tokio::test]
    async fn test_get_validator_set_for_epoch_without_static_validators() {
        // Create a datastore without static validators
        let datastore = DatastoreManager::create_in_memory().unwrap();
        
        // This should fail since there are no blocks for dynamic selection
        let result = get_validator_set_for_epoch_multi(&datastore, 0).await;
        assert!(result.is_err());
    }
}
