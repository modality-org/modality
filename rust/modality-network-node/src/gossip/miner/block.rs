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
    source_peer: Option<libp2p::PeerId>,
    datastore: std::sync::Arc<tokio::sync::Mutex<NetworkDatastore>>,
    sync_request_tx: Option<tokio::sync::mpsc::UnboundedSender<(libp2p::PeerId, String)>>,
    mining_update_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
) -> Result<()> {
    log::debug!("Received miner block gossip");
    
    // Parse the gossip message
    let gossip_msg: MinerBlockGossip = serde_json::from_str(&data)?;
    let miner_block = gossip_msg.to_miner_block();
    
    log::debug!("Gossip block: index={}, hash={}", miner_block.index, &miner_block.hash[..16]);
    
    // Check if we already have this exact block (by hash)
    {
        let ds = datastore.lock().await;
        if let Ok(Some(_)) = MinerBlock::find_by_hash(&ds, &miner_block.hash).await {
            log::debug!("Block with hash {} already exists, skipping", &miner_block.hash[..16]);
            return Ok(());
        }
    }
    
    // Track if we save a new block or update the chain tip
    let mut chain_tip_updated = false;
    let mut new_tip_index = None;
    
    // **FIRST**: Check if a block exists at this index (fork choice)
    // This must happen BEFORE parent validation to handle competing blocks correctly
    {
        let mut ds = datastore.lock().await;
        if let Some(existing) = MinerBlock::find_canonical_by_index(&ds, miner_block.index).await? {
            // We have a different block at the same index - this is a fork!
            // Apply fork choice: higher difficulty wins, lower hash breaks ties
            let new_difficulty = miner_block.get_difficulty_u128()?;
            let existing_difficulty = existing.get_difficulty_u128()?;
            
            let should_replace = if new_difficulty > existing_difficulty {
                true
            } else if new_difficulty == existing_difficulty {
                // Tiebreaker: lower hash wins (lexicographic comparison)
                miner_block.hash < existing.hash
            } else {
                false
            };
            
            if should_replace {
                log::info!("Fork choice: Replacing existing block {} (difficulty: {}, hash: {}) with gossiped block (difficulty: {}, hash: {})",
                    miner_block.index, existing_difficulty, &existing.hash[..16], new_difficulty, &miner_block.hash[..16]);
                
                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by gossiped block (difficulty: {}, hash: {})", new_difficulty, &miner_block.hash[..16]),
                    Some(miner_block.hash.clone())
                );
                orphaned.save(&mut ds).await?;
                
                // Save new block as canonical
                miner_block.save(&mut ds).await?;
                log::info!("Accepted gossiped block {} at index {}", &miner_block.hash[..16], miner_block.index);
                
                // Check if this updates the chain tip
                let current_tip = MinerBlock::find_all_canonical(&ds).await?
                    .into_iter()
                    .max_by_key(|b| b.index)
                    .map(|b| b.index);
                
                if let Some(tip) = current_tip {
                    chain_tip_updated = true;
                    new_tip_index = Some(tip);
                }
            } else {
                log::debug!("Existing block {} wins fork choice (existing difficulty: {}, hash: {} vs new difficulty: {}, hash: {})", 
                    miner_block.index, existing_difficulty, &existing.hash[..16], new_difficulty, &miner_block.hash[..16]);
            }
            
            // Fork handled - notify if needed and return
            drop(ds);
            if chain_tip_updated {
                if let Some(tip) = new_tip_index {
                    if let Some(ref tx) = mining_update_tx {
                        log::info!("📡 Chain tip updated to {} via gossip fork choice, notifying mining loop", tip);
                        let _ = tx.send(tip);
                    }
                }
            }
            return Ok(());
        }
    }
    
    // **SECOND**: Validate we have the parent block (chain continuity)
    if miner_block.index > 0 {
        let ds = datastore.lock().await;
        
        // Check if the parent exists by hash
        match MinerBlock::find_by_hash(&ds, &miner_block.previous_hash).await? {
            None => {
                log::warn!(
                    "Received block {} but missing parent block (prev_hash: {}). Orphan block detected!",
                    miner_block.index, 
                    &miner_block.previous_hash[..16]
                );
                
                // Check if this is a completely different chain by comparing genesis
                let our_genesis = MinerBlock::find_canonical_by_index(&ds, 0).await?;
                if let Some(genesis) = our_genesis {
                    log::warn!(
                        "⚠️  We have genesis block {} but received orphan from different chain.",
                        &genesis.hash[..16]
                    );
                } else {
                    log::info!("No local genesis - will need to sync from peers");
                }
                
                drop(ds);
                
                // Send sync request via channel if available
                if let Some(ref tx) = sync_request_tx {
                    if let Some(peer_id) = source_peer {
                        // Find the peer's address from bootstrappers
                        let peer_addr = bootstrappers.iter()
                            .find(|addr| {
                                addr.iter().any(|proto| matches!(proto, libp2p::multiaddr::Protocol::P2p(id) if id == peer_id))
                            })
                            .map(|addr| addr.to_string());
                        
                        if let Some(addr) = peer_addr {
                            // Add random delay (100-500ms) to avoid simultaneous sync attempts from both nodes
                            let delay_ms = 100 + (rand::random::<u64>() % 400);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            
                            log::info!("📡 Orphan detected from peer {} - requesting chain sync (after {}ms delay)", peer_id, delay_ms);
                            if let Err(e) = tx.send((peer_id, addr)) {
                                log::warn!("Failed to send sync request: {}", e);
                            }
                        } else {
                            log::warn!("Could not find address for peer {} in bootstrappers", peer_id);
                        }
                    }
                } else {
                    log::debug!("Sync request channel not initialized yet");
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
                
                // CRITICAL: Also check if there's a DIFFERENT canonical block at index-1
                // This prevents accepting a block that builds on an orphaned parent
                if let Ok(Some(canonical_at_parent_index)) = MinerBlock::find_canonical_by_index(&ds, miner_block.index - 1).await {
                    if canonical_at_parent_index.hash != miner_block.previous_hash {
                        log::warn!(
                            "⚠️  Block {} builds on orphaned parent. Canonical block at index {} has hash {}, but this block expects {}. Rejecting.",
                            miner_block.index,
                            miner_block.index - 1,
                            &canonical_at_parent_index.hash[..16],
                            &miner_block.previous_hash[..16]
                        );
                        return Ok(());
                    }
                }
                
                log::debug!("Parent block validated for block {}", miner_block.index);
            }
        }
        drop(ds); // Release lock
    }
    
    // At this point, we've validated the block has a valid parent and doesn't conflict with existing blocks
    // Save it and notify the mining loop
    {
        let mut ds = datastore.lock().await;
        log::info!("Accepting new gossiped block {} at index {}", &miner_block.hash[..16], miner_block.index);
        miner_block.save(&mut ds).await?;
        
        // Check if this extends the chain tip
        let current_tip = MinerBlock::find_all_canonical(&ds).await?
            .into_iter()
            .max_by_key(|b| b.index)
            .map(|b| b.index);
        
        if let Some(tip) = current_tip {
            if let Some(ref tx) = mining_update_tx {
                log::info!("📡 Chain tip extended to {} via gossip, notifying mining loop", tip);
                let _ = tx.send(tip);
            }
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

