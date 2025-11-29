//! Common ancestor finding using efficient binary search.
//!
//! This module implements an efficient algorithm for finding the common
//! ancestor between local and remote chains using exponential search
//! followed by binary search for O(log n) complexity.

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::constants::{MAX_CHECKPOINTS_PER_REQUEST, REQRES_TIMEOUT_SECS};
use crate::reqres;

/// Result of common ancestor search
#[derive(Debug, Clone)]
pub struct AncestorSearchResult {
    /// Index of the common ancestor (None if no common ancestor)
    pub ancestor_index: Option<u64>,
    /// Remote chain length
    pub remote_chain_length: u64,
    /// Remote chain cumulative difficulty
    pub remote_cumulative_difficulty: u128,
}

/// Wait for a reqres response using channels (no swarm lock contention).
pub async fn wait_for_reqres_response(
    node_reqres_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
    request_id: libp2p::request_response::OutboundRequestId,
) -> Result<reqres::Response> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    {
        let mut txs = node_reqres_txs.lock().await;
        txs.insert(request_id, tx);
    }
    
    rx.await.map_err(|_| anyhow::anyhow!("Response channel closed"))
}

/// Efficiently find the common ancestor between local and remote chains using binary search.
///
/// This function uses the `/data/miner_block/find_ancestor` route to iteratively find
/// the highest block index where both chains agree, using an exponential search followed
/// by binary search for O(log n) complexity.
///
/// # Arguments
/// * `swarm` - The swarm for making requests
/// * `peer_addr` - The peer address to query
/// * `datastore` - Local datastore to get our chain
/// * `reqres_response_txs` - Channel map for response routing
///
/// # Returns
/// * `Ok(AncestorSearchResult)` with ancestor info and peer chain metrics
/// * `Err(_)` - Error during the search
pub async fn find_common_ancestor_efficient(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<AncestorSearchResult> {
    use libp2p::multiaddr::Multiaddr;
    
    log::info!("🔍 Finding common ancestor with peer using efficient binary search");
    
    // Load our local canonical chain
    let local_blocks = {
        let ds = datastore.lock().await;
        MinerBlock::find_all_canonical_multi(&ds).await?
    };
    
    // Parse peer address
    let ma: Multiaddr = peer_addr.parse()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Invalid peer address - missing PeerID");
    };
    
    if local_blocks.is_empty() {
        log::info!("Local chain is empty, no common ancestor");
        
        // Still need to get the peer's chain info
        let (remote_chain_length, remote_cumulative_difficulty) = 
            get_peer_chain_info(swarm, &target_peer_id, reqres_response_txs).await?;
        
        return Ok(AncestorSearchResult {
            ancestor_index: None,
            remote_chain_length,
            remote_cumulative_difficulty,
        });
    }
    
    let local_chain_length = local_blocks.len() as u64;
    log::debug!("Local chain length: {}", local_chain_length);
    
    // Step 1: Exponential search to find an upper bound
    let checkpoints = build_exponential_checkpoints(&local_blocks, local_chain_length);
    log::debug!("Phase 1: Exponential search with {} checkpoints", checkpoints.len());
    
    // Make the initial request
    let (highest_match, matches, remote_chain_length, remote_cumulative_difficulty) = 
        send_find_ancestor_request(swarm, &target_peer_id, &checkpoints, reqres_response_txs).await?;
    
    log::info!(
        "Remote chain length: {}, cumulative difficulty: {}, Initial highest match: {:?}",
        remote_chain_length, remote_cumulative_difficulty, highest_match
    );
    
    // If no match at all, chains have no common ancestor
    if highest_match.is_none() {
        log::warn!("No common blocks found - chains have completely diverged (different genesis?)");
        return Ok(AncestorSearchResult {
            ancestor_index: None,
            remote_chain_length,
            remote_cumulative_difficulty,
        });
    }
    
    let mut highest_match_idx = highest_match.unwrap();
    
    // Step 2: Binary search to find the exact divergence point
    let (search_low, search_high) = determine_binary_search_bounds(
        highest_match_idx,
        local_chain_length,
        &matches,
    );
    
    log::debug!("Phase 2: Batched binary search between {} and {}", search_low, search_high);
    
    // Perform batched binary search
    highest_match_idx = batched_binary_search(
        swarm,
        &target_peer_id,
        &local_blocks,
        search_low,
        search_high,
        highest_match_idx,
        reqres_response_txs,
    ).await?;
    
    log::info!("✅ Found common ancestor at block index {}", highest_match_idx);
    
    Ok(AncestorSearchResult {
        ancestor_index: Some(highest_match_idx),
        remote_chain_length,
        remote_cumulative_difficulty,
    })
}

/// Get peer chain info (length and cumulative difficulty).
async fn get_peer_chain_info(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    target_peer_id: &libp2p::PeerId,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<(u64, u128)> {
    let request = reqres::Request {
        path: "/data/miner_block/chain_info".to_string(),
        data: None,
    };
    
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock.behaviour_mut().reqres.send_request(target_peer_id, request)
    };
    
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        wait_for_reqres_response(reqres_response_txs, request_id)
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            log::warn!("Failed to get chain info from peer: {}", e);
            return Ok((0, 0));
        }
        Err(_) => {
            log::warn!("Timeout waiting for chain info from peer");
            return Ok((0, 0));
        }
    };
    
    if !response.ok {
        log::warn!("Peer returned error for chain info request");
        return Ok((0, 0));
    }
    
    let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in chain info response"))?;
    let chain_length = data.get("chain_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cumulative_difficulty = data.get("cumulative_difficulty")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);
    
    log::info!("Peer chain: {} blocks, cumulative difficulty: {}", chain_length, cumulative_difficulty);
    
    Ok((chain_length, cumulative_difficulty))
}

/// Build exponential checkpoints for initial search.
fn build_exponential_checkpoints(
    local_blocks: &[MinerBlock],
    local_chain_length: u64,
) -> Vec<(u64, String)> {
    let mut checkpoints = Vec::new();
    let mut step = 0;
    
    loop {
        let index = if step == 0 {
            local_chain_length.saturating_sub(1)
        } else if step == 1 {
            local_chain_length.saturating_sub(2)
        } else {
            local_chain_length.saturating_sub(1 << step)
        };
        
        if index >= local_chain_length {
            break;
        }
        
        if let Some(block) = local_blocks.iter().find(|b| b.index == index) {
            checkpoints.push((block.index, block.hash.clone()));
        }
        
        if index == 0 {
            break;
        }
        
        step += 1;
    }
    
    checkpoints
}

/// Send find_ancestor request and parse response.
async fn send_find_ancestor_request(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    target_peer_id: &libp2p::PeerId,
    checkpoints: &[(u64, String)],
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<(Option<u64>, Vec<serde_json::Value>, u64, u128)> {
    let request = reqres::Request {
        path: "/data/miner_block/find_ancestor".to_string(),
        data: Some(serde_json::json!({
            "check_points": checkpoints.iter().map(|(idx, hash)| {
                serde_json::json!({
                    "index": idx,
                    "hash": hash
                })
            }).collect::<Vec<_>>()
        })),
    };
    
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock.behaviour_mut().reqres.send_request(target_peer_id, request)
    };
    
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(REQRES_TIMEOUT_SECS / 3),
        wait_for_reqres_response(reqres_response_txs, request_id)
    ).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return Err(e),
        Err(_) => anyhow::bail!("Timeout waiting for find_ancestor response"),
    };
    
    if !response.ok {
        anyhow::bail!("Peer returned error: {:?}", response.errors);
    }
    
    let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in response"))?;
    let highest_match = data.get("highest_match").and_then(|v| v.as_u64());
    let remote_chain_length = data.get("chain_length").and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing chain_length in response"))?;
    let remote_cumulative_difficulty = data.get("cumulative_difficulty")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid cumulative_difficulty in response"))?;
    
    let matches = data.get("matches")
        .and_then(|v| v.as_array())
        .map(|v| v.clone())
        .unwrap_or_default();
    
    Ok((highest_match, matches, remote_chain_length, remote_cumulative_difficulty))
}

/// Determine the bounds for binary search based on initial results.
fn determine_binary_search_bounds(
    highest_match: u64,
    local_chain_length: u64,
    matches: &[serde_json::Value],
) -> (u64, u64) {
    let mut search_low = highest_match;
    let mut search_high = local_chain_length - 1;
    
    // Find the first non-matching index that's higher than highest_match
    for match_info in matches {
        let idx = match_info.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
        let matches_val = match_info.get("matches").and_then(|v| v.as_bool()).unwrap_or(false);
        
        if !matches_val && idx > highest_match && idx < search_high {
            search_high = idx;
        }
    }
    
    (search_low, search_high)
}

/// Perform batched binary search to find exact common ancestor.
async fn batched_binary_search(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    target_peer_id: &libp2p::PeerId,
    local_blocks: &[MinerBlock],
    mut search_low: u64,
    mut search_high: u64,
    mut highest_match_idx: u64,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<u64> {
    while search_low < search_high && search_high - search_low > 1 {
        let range_size = (search_high - search_low) as usize;
        
        // Generate checkpoints spanning the search range
        let checkpoints = generate_search_checkpoints(
            local_blocks,
            search_low,
            search_high,
            range_size,
        );
        
        if checkpoints.is_empty() {
            log::debug!("No local blocks in search range, narrowing...");
            break;
        }
        
        log::debug!(
            "Batched binary search: sending {} checkpoints (range: {} to {})",
            checkpoints.len(), search_low, search_high
        );
        
        // Send request
        let (_, matches, _, _) = send_find_ancestor_request(
            swarm,
            target_peer_id,
            &checkpoints,
            reqres_response_txs,
        ).await?;
        
        // Update bounds based on results
        let (new_highest, new_low, new_high) = process_binary_search_results(
            &matches,
            highest_match_idx,
            search_low,
            search_high,
        );
        
        highest_match_idx = new_highest;
        search_low = new_low;
        search_high = new_high;
        
        // If we checked every block in a small range, we're done
        if range_size <= MAX_CHECKPOINTS_PER_REQUEST {
            break;
        }
    }
    
    Ok(highest_match_idx)
}

/// Generate checkpoints for binary search.
fn generate_search_checkpoints(
    local_blocks: &[MinerBlock],
    search_low: u64,
    search_high: u64,
    range_size: usize,
) -> Vec<(u64, String)> {
    let mut checkpoints = Vec::new();
    
    if range_size <= MAX_CHECKPOINTS_PER_REQUEST {
        // Small range: check every block
        for idx in (search_low + 1)..=search_high {
            if let Some(block) = local_blocks.iter().find(|b| b.index == idx) {
                checkpoints.push((block.index, block.hash.clone()));
            }
        }
    } else {
        // Large range: distribute checkpoints evenly
        let step = (range_size / MAX_CHECKPOINTS_PER_REQUEST).max(1);
        
        let mut idx = search_low + 1;
        while idx <= search_high && checkpoints.len() < MAX_CHECKPOINTS_PER_REQUEST {
            if let Some(block) = local_blocks.iter().find(|b| b.index == idx) {
                checkpoints.push((block.index, block.hash.clone()));
            }
            idx += step as u64;
        }
        
        // Always include search_high
        if let Some(block) = local_blocks.iter().find(|b| b.index == search_high) {
            if checkpoints.last().map(|(i, _)| *i) != Some(search_high) {
                checkpoints.push((block.index, block.hash.clone()));
            }
        }
    }
    
    checkpoints
}

/// Process binary search results and update bounds.
fn process_binary_search_results(
    matches: &[serde_json::Value],
    current_highest: u64,
    current_low: u64,
    current_high: u64,
) -> (u64, u64, u64) {
    let mut highest_match_idx = current_highest;
    let mut search_low = current_low;
    let mut search_high = current_high;
    
    let mut batch_highest_match: Option<u64> = None;
    let mut batch_lowest_non_match: Option<u64> = None;
    
    for match_info in matches {
        let idx = match_info.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
        let matches_val = match_info.get("matches").and_then(|v| v.as_bool()).unwrap_or(false);
        
        if matches_val {
            if batch_highest_match.is_none() || idx > batch_highest_match.unwrap() {
                batch_highest_match = Some(idx);
            }
        } else if batch_lowest_non_match.is_none() || idx < batch_lowest_non_match.unwrap() {
            batch_lowest_non_match = Some(idx);
        }
    }
    
    if let Some(highest) = batch_highest_match {
        if highest > highest_match_idx {
            highest_match_idx = highest;
        }
        search_low = highest;
        log::debug!("Batch found match at {}, new search_low = {}", highest, search_low);
    }
    
    if let Some(lowest_non_match) = batch_lowest_non_match {
        if lowest_non_match < search_high {
            search_high = lowest_non_match;
            log::debug!("Batch found non-match at {}, new search_high = {}", lowest_non_match, search_high);
        }
    }
    
    (highest_match_idx, search_low, search_high)
}

