use anyhow::Result;
use modality_network_datastore::Model;
use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::MinerBlock;
use serde::{Deserialize, Serialize};

pub const TOPIC: &str = "/miner/block";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerBlockGossip {
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    pub nominated_peer_id: String,
    pub previous_hash: String,
    pub difficulty: String,
    pub nonce: String,
    pub timestamp: String,
    pub miner_number: u64,
}

impl MinerBlockGossip {
    pub fn from_miner_block(block: &MinerBlock) -> Self {
        Self {
            hash: block.hash.clone(),
            index: block.index,
            epoch: block.epoch,
            nominated_peer_id: block.nominated_peer_id.clone(),
            previous_hash: block.previous_hash.clone(),
            difficulty: block.difficulty.clone(),
            nonce: block.nonce.clone(),
            timestamp: block.timestamp.to_string(),
            miner_number: block.miner_number,
        }
    }

    pub fn to_miner_block(&self) -> MinerBlock {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = self.timestamp.parse::<i64>().unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        });
        
        let nonce = self.nonce.parse::<u128>().unwrap_or(0);
        let difficulty = self.difficulty.parse::<u128>().unwrap_or(1000);
        
        MinerBlock::new_canonical(
            self.hash.clone(),
            self.index,
            self.epoch,
            timestamp,
            self.previous_hash.clone(),
            String::new(), // data_hash - will be set by the model
            nonce,
            difficulty,
            self.nominated_peer_id.clone(),
            self.miner_number,
        )
    }
}

/// Handler for incoming miner block gossip messages
pub async fn handler(
    data: String, 
    datastore: &mut NetworkDatastore,
    sync_trigger_tx: Option<tokio::sync::broadcast::Sender<u64>>,
) -> Result<()> {
    log::debug!("Received miner block gossip");
    
    // Parse the gossip message
    let gossip_msg: MinerBlockGossip = serde_json::from_str(&data)?;
    let miner_block = gossip_msg.to_miner_block();
    
    log::debug!("Gossip block: index={}, hash={}", miner_block.index, &miner_block.hash[..16]);
    
    // Check if we already have this exact block (by hash)
    if let Ok(Some(_)) = MinerBlock::find_by_hash(datastore, &miner_block.hash).await {
        log::debug!("Block with hash {} already exists, skipping", &miner_block.hash[..16]);
        return Ok(());
    }
    
    // Validate we have the parent block (chain continuity)
    if miner_block.index > 0 {
        match MinerBlock::find_by_hash(datastore, &miner_block.previous_hash).await? {
            None => {
                log::warn!(
                    "Received block {} but missing parent block (prev_hash: {}). Orphan block detected - triggering sync!",
                    miner_block.index, 
                    &miner_block.previous_hash[..16]
                );
                
                // Trigger sync to get missing blocks
                if let Some(tx) = sync_trigger_tx {
                    if let Err(e) = tx.send(miner_block.index) {
                        log::warn!("Failed to trigger sync: {}", e);
                    } else {
                        log::info!("🔄 Sync triggered for missing blocks up to index {}", miner_block.index);
                    }
                }
                
                // Don't save orphan blocks - they can't be validated
                return Ok(());
            }
            Some(parent) => {
                // Validate parent is canonical
                if !parent.is_canonical {
                    log::warn!("Parent block {} is not canonical, rejecting gossiped block {}", 
                        parent.index, miner_block.index);
                    return Ok(());
                }
                
                // Validate parent is at expected index
                if parent.index != miner_block.index - 1 {
                    log::warn!(
                        "Parent block index mismatch: expected {}, got {}. Rejecting block {}",
                        miner_block.index - 1,
                        parent.index,
                        miner_block.index
                    );
                    return Ok(());
                }
                
                log::debug!("Parent block validated for block {}", miner_block.index);
            }
        }
    }
    
    // Check if a block exists at this index (fork choice)
    match MinerBlock::find_canonical_by_index(datastore, miner_block.index).await? {
        Some(existing) => {
            // Apply fork choice: higher difficulty (more work) wins
            let new_difficulty = miner_block.get_difficulty_u128()?;
            let existing_difficulty = existing.get_difficulty_u128()?;
            
            if new_difficulty > existing_difficulty {
                log::info!("Fork choice: Replacing existing block {} (difficulty: {}) with gossiped block (difficulty: {})",
                    miner_block.index, existing_difficulty, new_difficulty);
                
                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by gossiped block with higher difficulty ({} vs {})", new_difficulty, existing_difficulty),
                    Some(miner_block.hash.clone())
                );
                orphaned.save(datastore).await?;
                
                // Save new block as canonical
                miner_block.save(datastore).await?;
                log::info!("Accepted gossiped block {} at index {}", &miner_block.hash[..16], miner_block.index);
            } else {
                log::debug!("Existing block {} has equal or higher difficulty (existing: {}, new: {}), keeping it", 
                    miner_block.index, existing_difficulty, new_difficulty);
            }
        }
        None => {
            // No block at this index, save it
            log::info!("Accepting new gossiped block {} at index {}", &miner_block.hash[..16], miner_block.index);
            miner_block.save(datastore).await?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_miner_block_gossip_serialization() {
        let gossip = MinerBlockGossip {
            hash: "abc123".to_string(),
            index: 1,
            epoch: 0,
            nominated_peer_id: "peer1".to_string(),
            previous_hash: "genesis".to_string(),
            difficulty: "1000".to_string(),
            nonce: "12345".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            miner_number: 42,
        };

        let json = serde_json::to_string(&gossip).unwrap();
        let deserialized: MinerBlockGossip = serde_json::from_str(&json).unwrap();

        assert_eq!(gossip.hash, deserialized.hash);
        assert_eq!(gossip.index, deserialized.index);
    }

    #[test]
    fn test_miner_block_conversion() {
        let gossip = MinerBlockGossip {
            hash: "abc123".to_string(),
            index: 1,
            epoch: 0,
            nominated_peer_id: "peer1".to_string(),
            previous_hash: "genesis".to_string(),
            difficulty: "1000".to_string(),
            nonce: "12345".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            miner_number: 42,
        };

        let miner_block = gossip.to_miner_block();
        assert_eq!(miner_block.hash, gossip.hash);
        assert_eq!(miner_block.index, gossip.index);
        assert!(miner_block.is_canonical);

        let gossip2 = MinerBlockGossip::from_miner_block(&miner_block);
        assert_eq!(gossip2.hash, gossip.hash);
        assert_eq!(gossip2.index, gossip.index);
    }
}

