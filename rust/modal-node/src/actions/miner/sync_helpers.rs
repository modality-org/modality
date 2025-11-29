//! Sync helper functions for the miner.
//!
//! This module provides miner-specific sync helpers and re-exports
//! common sync functionality from observer.

use anyhow::Result;
use libp2p::gossipsub::IdentTopic;
use modal_datastore::models::MinerBlock;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::gossip;
use crate::node::Node;

// Re-export observer's sync functions for use by miner
pub use crate::actions::observer::{
    sync_from_peers,
    request_chain_info_impl,
    find_common_ancestor_efficient,
};

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
