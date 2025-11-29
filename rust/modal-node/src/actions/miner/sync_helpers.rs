//! Sync helper functions for the miner.
//!
//! This module contains miner-specific sync helpers. For common sync
//! functionality, see `crate::actions::observer::sync`.

use anyhow::Result;
use libp2p::gossipsub::IdentTopic;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::fork_choice::compare_chains;
use crate::chain::metrics::calculate_cumulative_difficulty;
use crate::gossip;
use crate::node::{Node, IgnoredPeerInfo};
use crate::reqres;

// Re-export observer's sync_from_peers for use by miner
pub use crate::actions::observer::sync_from_peers;

/// Announce our chain tip to connected peers.
/// This is miner-specific as observers don't need to announce their chain.
pub async fn announce_chain_tip(node: &Node) -> Result<()> {
    let tip_block = {
        let mgr = node.datastore_manager.lock().await;
        MinerBlock::find_all_canonical_multi(&mgr).await?
            .into_iter()
            .max_by_key(|b| b.index)
    };
    
    if let Some(block) = tip_block {
        log::info!("Announcing chain tip: block {} (index: {})", &block.hash[..16], block.index);
        
        let gossip_msg = gossip::miner::block::MinerBlockGossip::from_miner_block(&block);
        let topic = IdentTopic::new(gossip::miner::block::TOPIC);
        let json = serde_json::to_string(&gossip_msg)?;
        
        let mut swarm_lock = node.swarm.lock().await;
        match swarm_lock.behaviour_mut().gossipsub.publish(topic, json.as_bytes()) {
            Ok(_) => {
                log::info!("✓ Announced our chain tip (block {}) to peers", block.index);
            }
            Err(e) => {
                log::debug!("Could not gossip chain tip: {}", e);
            }
        }
    } else {
        log::info!("No blocks to announce (empty chain)");
    }
    
    Ok(())
}

/// Request chain info from a peer and perform sync if their chain has higher cumulative difficulty.
pub async fn request_chain_info_impl(
    peer_id: libp2p::PeerId,
    peer_addr: String,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<()> {
    // Check if peer is ignored
    {
        let ignored = ignored_peers.lock().await;
        if let Some(info) = ignored.get(&peer_id) {
            if std::time::Instant::now() < info.ignore_until {
                log::debug!("Peer {} is ignored, skipping chain info request", peer_id);
                return Ok(());
            }
        }
    }
    
    log::info!("🔄 Syncing with peer {} using efficient find_ancestor", peer_id);
    
    // Find common ancestor
    let (common_ancestor, peer_chain_length, peer_cumulative_difficulty) = 
        find_common_ancestor_efficient(&swarm, peer_addr.clone(), &datastore, &reqres_response_txs).await?;
    
    // Determine blocks to request
    let from_index = match common_ancestor {
        Some(ancestor_index) => {
            log::info!("✓ Found common ancestor at index {}", ancestor_index);
            ancestor_index + 1
        }
        None => {
            log::warn!("⚠️  No common ancestor found - chains completely diverged");
            0
        }
    };
    
    // Get local chain info
    let (local_cumulative_difficulty, local_chain_length) = {
        let ds = datastore.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        let local_difficulty = calculate_cumulative_difficulty(&blocks);
        (local_difficulty, blocks.len() as u64)
    };
    
    // Compare chains
    let comparison = compare_chains(
        local_cumulative_difficulty,
        local_chain_length,
        peer_cumulative_difficulty,
        peer_chain_length,
    );
    
    log::info!(
        "Chain comparison: Local (length: {}, difficulty: {}) vs Peer (length: {}, difficulty: {})",
        local_chain_length, local_cumulative_difficulty,
        peer_chain_length, peer_cumulative_difficulty
    );
    
    use crate::chain::fork_choice::ForkChoiceResult;
    if comparison.result != ForkChoiceResult::AdoptRemote {
        log::info!("Keeping local chain: {}", comparison.reason);
        let ds = datastore.lock().await;
        let _ = MinerBlock::delete_all_pending_multi(&ds).await;
        return Ok(());
    }
    
    log::info!("✅ Peer chain has higher cumulative difficulty - adopting it");
    
    // Request blocks from peer
    let all_blocks = request_blocks_from_peer(
        &swarm,
        &peer_addr,
        from_index,
        peer_chain_length,
        &reqres_response_txs,
    ).await?;
    
    if all_blocks.is_empty() {
        log::info!("No blocks received from peer");
        return Ok(());
    }
    
    // Validate and adopt blocks
    adopt_peer_blocks(
        &datastore,
        all_blocks,
        common_ancestor,
        peer_cumulative_difficulty,
        local_cumulative_difficulty,
    ).await?;
    
    Ok(())
}

/// Request blocks from a peer
async fn request_blocks_from_peer(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    from_index: u64,
    to_index: u64,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<Vec<MinerBlock>> {
    use crate::sync::block_range::request_all_blocks_in_range;
    
    log::info!("📥 Requesting blocks from index {} onwards from peer", from_index);
    
    request_all_blocks_in_range(swarm, peer_addr, from_index, to_index, reqres_response_txs).await
}

/// Adopt blocks from peer after validation
async fn adopt_peer_blocks(
    datastore: &Arc<Mutex<DatastoreManager>>,
    mut all_blocks: Vec<MinerBlock>,
    common_ancestor: Option<u64>,
    peer_cumulative_difficulty: u128,
    local_cumulative_difficulty: u128,
) -> Result<()> {
    // Sort blocks
    all_blocks.sort_by_key(|b| b.index);
    
    // Validate chain
    use crate::chain::reorg::validate_block_chain;
    validate_block_chain(&all_blocks)?;
    
    log::info!("✓ Peer chain validation passed");
    
    // Orphan local blocks after ancestor and adopt peer blocks
    let ancestor_index = all_blocks.first().map(|b| b.index.saturating_sub(1)).unwrap_or(0);
    
    {
        let ds = datastore.lock().await;
        
        // Orphan local blocks after ancestor
        let local_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        
        for local in &local_blocks {
            if local.index > ancestor_index {
                let competing_hash = all_blocks.iter()
                    .find(|b| b.index == local.index)
                    .map(|b| b.hash.clone());
                
                log::info!("Orphaning local block {} at index {}", &local.hash[..16], local.index);
                let mut orphaned = local.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by peer chain with higher cumulative difficulty ({} vs {})",
                        peer_cumulative_difficulty, local_cumulative_difficulty),
                    competing_hash
                );
                orphaned.save_to_active(&ds).await?;
            }
        }
        
        // Save peer blocks
        for block in &all_blocks {
            block.save_to_active(&ds).await?;
        }
    }
    
    log::info!("🎉 Successfully adopted peer's chain with {} blocks!", all_blocks.len());
    
    Ok(())
}

/// Efficiently find the common ancestor between local and remote chains using binary search.
pub async fn find_common_ancestor_efficient(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<(Option<u64>, u64, u128)> {
    // Delegate to the sync module implementation
    let result = crate::sync::common_ancestor::find_common_ancestor_efficient(
        swarm,
        peer_addr,
        datastore,
        reqres_response_txs,
    ).await?;
    
    Ok((result.ancestor_index, result.remote_chain_length, result.remote_cumulative_difficulty))
}

