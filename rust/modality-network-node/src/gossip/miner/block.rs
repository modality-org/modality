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
pub async fn handler(data: String, datastore: &mut NetworkDatastore) -> Result<()> {
    log::info!("Received miner block gossip: {}", data);
    
    // Parse the gossip message
    let gossip_msg: MinerBlockGossip = serde_json::from_str(&data)?;
    
    // Convert to MinerBlock
    let miner_block = gossip_msg.to_miner_block();
    
    // Check if we already have this block
    match MinerBlock::find_by_hash(datastore, &miner_block.hash).await {
        Ok(Some(existing_block)) => {
            log::debug!("Miner block {} already exists, skipping", existing_block.hash);
            return Ok(());
        }
        Ok(None) => {
            // Block doesn't exist, save it
            log::info!("Saving new miner block {} at index {}", miner_block.hash, miner_block.index);
            miner_block.save(datastore).await?;
        }
        Err(e) => {
            log::error!("Error checking for existing miner block: {:?}", e);
            return Err(e);
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
        assert!(miner_block.canonical);

        let gossip2 = MinerBlockGossip::from_miner_block(&miner_block);
        assert_eq!(gossip2.hash, gossip.hash);
        assert_eq!(gossip2.index, gossip.index);
    }
}

