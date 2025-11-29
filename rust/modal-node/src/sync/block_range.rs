//! Block range request utilities.
//!
//! This module provides functions for requesting ranges of blocks from peers.

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::constants::{MAX_BLOCKS_PER_RANGE_REQUEST, REQRES_TIMEOUT_SECS};
use crate::reqres;
use crate::sync::common_ancestor::wait_for_reqres_response;

/// Result of a block range request
#[derive(Debug, Clone)]
pub struct BlockRangeResult {
    /// Blocks received
    pub blocks: Vec<MinerBlock>,
    /// Whether there are more blocks available
    pub has_more: bool,
    /// Next index to request from (if has_more is true)
    pub next_from_index: u64,
}

/// Request a range of blocks from a peer.
///
/// # Arguments
/// * `swarm` - The swarm for making requests
/// * `peer_addr` - The peer address to query
/// * `from_index` - Start index (inclusive)
/// * `to_index` - End index (inclusive)
/// * `reqres_response_txs` - Channel map for response routing
///
/// # Returns
/// BlockRangeResult with the received blocks
pub async fn request_block_range(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    from_index: u64,
    to_index: u64,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<BlockRangeResult> {
    use libp2p::multiaddr::Multiaddr;
    
    let ma: Multiaddr = peer_addr.parse()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Invalid peer address - missing PeerID");
    };
    
    log::debug!("Requesting blocks {}..{} from peer", from_index, to_index);
    
    let request = reqres::Request {
        path: "/data/miner_block/range".to_string(),
        data: Some(serde_json::json!({
            "from_index": from_index,
            "to_index": to_index
        })),
    };
    
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
    };
    
    log::debug!("Block range request sent with ID: {:?}", request_id);
    
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(REQRES_TIMEOUT_SECS),
        wait_for_reqres_response(reqres_response_txs, request_id)
    ).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            log::warn!("Failed to get block range: {}", e);
            return Ok(BlockRangeResult {
                blocks: vec![],
                has_more: false,
                next_from_index: from_index,
            });
        }
        Err(_) => {
            log::warn!("Block range request timed out");
            return Ok(BlockRangeResult {
                blocks: vec![],
                has_more: false,
                next_from_index: from_index,
            });
        }
    };
    
    if !response.ok {
        log::warn!("Peer returned error for block range: {:?}", response.errors);
        return Ok(BlockRangeResult {
            blocks: vec![],
            has_more: false,
            next_from_index: from_index,
        });
    }
    
    let Some(ref data) = response.data else {
        log::warn!("Peer returned no data for block range");
        return Ok(BlockRangeResult {
            blocks: vec![],
            has_more: false,
            next_from_index: from_index,
        });
    };
    
    // Parse blocks from response
    let Some(blocks_json) = data.get("blocks").and_then(|b| b.as_array()) else {
        log::warn!("No blocks array in response");
        return Ok(BlockRangeResult {
            blocks: vec![],
            has_more: false,
            next_from_index: from_index,
        });
    };
    
    let mut blocks = Vec::with_capacity(blocks_json.len());
    for block_json in blocks_json {
        match serde_json::from_value(block_json.clone()) {
            Ok(block) => blocks.push(block),
            Err(e) => {
                log::warn!("Failed to parse block: {}", e);
            }
        }
    }
    
    let has_more = data.get("has_more").and_then(|v| v.as_bool()).unwrap_or(false);
    let next_from_index = from_index + blocks.len() as u64;
    
    log::info!(
        "Received {} blocks from peer (indices {}..{})",
        blocks.len(),
        from_index,
        from_index + blocks.len().saturating_sub(1) as u64
    );
    
    Ok(BlockRangeResult {
        blocks,
        has_more,
        next_from_index,
    })
}

/// Request all blocks in a range, handling pagination.
///
/// # Arguments
/// * `swarm` - The swarm for making requests
/// * `peer_addr` - The peer address to query
/// * `from_index` - Start index (inclusive)
/// * `to_index` - End index (inclusive)
/// * `reqres_response_txs` - Channel map for response routing
///
/// # Returns
/// All blocks in the range
pub async fn request_all_blocks_in_range(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    from_index: u64,
    to_index: u64,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<Vec<MinerBlock>> {
    let mut all_blocks = Vec::new();
    let mut current_from = from_index;
    
    loop {
        let result = request_block_range(
            swarm,
            peer_addr,
            current_from,
            to_index,
            reqres_response_txs,
        ).await?;
        
        if result.blocks.is_empty() {
            break;
        }
        
        all_blocks.extend(result.blocks);
        
        if !result.has_more {
            break;
        }
        
        current_from = result.next_from_index;
    }
    
    log::info!("Total blocks received: {}", all_blocks.len());
    Ok(all_blocks)
}

/// Save received blocks to datastore with fork choice.
///
/// # Arguments
/// * `mgr` - Datastore manager
/// * `blocks` - Blocks to save
///
/// # Returns
/// Number of blocks saved
pub async fn save_blocks_with_fork_choice(
    mgr: &mut DatastoreManager,
    blocks: &[MinerBlock],
) -> Result<usize> {
    use crate::chain::fork_choice::should_replace_block;
    
    let mut saved_count = 0;
    
    for block in blocks {
        // Check if we already have this block
        if MinerBlock::find_by_hash_multi(mgr, &block.hash).await?.is_some() {
            continue;
        }
        
        // Check for existing block at this index
        if let Some(existing) = MinerBlock::find_canonical_by_index_simple(mgr, block.index).await? {
            // Apply fork choice
            if should_replace_block(block, &existing) {
                log::info!(
                    "Fork choice during sync: Replacing block {} with synced block",
                    block.index
                );
                
                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by synced block with better fork choice"),
                    Some(block.hash.clone())
                );
                orphaned.save_to_active(mgr).await?;
                
                // Save new block
                block.save_to_active(mgr).await?;
                saved_count += 1;
            } else {
                log::debug!("Existing block {} wins fork choice, skipping synced block", block.index);
            }
        } else {
            // No existing block, check parent
            if block.index > 0 {
                match MinerBlock::find_by_hash_multi(mgr, &block.previous_hash).await? {
                    Some(parent) if parent.is_canonical => {
                        block.save_to_active(mgr).await?;
                        saved_count += 1;
                        log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                    }
                    Some(_) => {
                        log::warn!("Parent block {} is not canonical, skipping block {}", &block.previous_hash[..16], block.index);
                    }
                    None => {
                        log::warn!("Cannot save block {} - missing parent", block.index);
                    }
                }
            } else {
                // Genesis block
                block.save_to_active(mgr).await?;
                saved_count += 1;
                log::debug!("Saved synced genesis block {}", &block.hash[..16]);
            }
        }
    }
    
    Ok(saved_count)
}

