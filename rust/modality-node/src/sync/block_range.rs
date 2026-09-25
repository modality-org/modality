//! Block range request utilities.
//!
//! This module provides functions for requesting ranges of blocks from peers.

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::constants::REQRES_TIMEOUT_SECS;
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
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
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
        swarm_lock
            .behaviour_mut()
            .reqres
            .send_request(&target_peer_id, request)
    };

    log::debug!("Block range request sent with ID: {:?}", request_id);

    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(REQRES_TIMEOUT_SECS),
        wait_for_reqres_response(reqres_response_txs, request_id),
    )
    .await
    {
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

    let has_more = data
        .get("has_more")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // The server pages by index (`to_index` is the last index this page
    // covered). Stepping by row count skips indexes when a page contains
    // two canonical rows for one index.
    let covered_to = data.get("to_index").and_then(|v| v.as_u64());
    let next_from_index = next_range_index(from_index, blocks.len(), covered_to);

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
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
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
        )
        .await?;

        if result.blocks.is_empty() && !result.has_more {
            break;
        }

        all_blocks.extend(result.blocks);

        if !result.has_more {
            break;
        }

        if result.next_from_index <= current_from || result.next_from_index > to_index {
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
        if MinerBlock::find_by_hash_multi(mgr, &block.hash)
            .await?
            .is_some()
        {
            continue;
        }

        // Check for existing block at this index
        if let Some(existing) = MinerBlock::find_canonical_by_index_simple(mgr, block.index).await?
        {
            // Apply fork choice
            if should_replace_block(block, &existing) {
                log::info!(
                    "Fork choice during sync: Replacing block {} with synced block",
                    block.index
                );

                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    "Replaced by synced block with better fork choice".to_string(),
                    Some(block.hash.clone()),
                );
                orphaned.save_to_active(mgr).await?;

                // Save new block
                block.save_to_active(mgr).await?;
                saved_count += 1;
            } else {
                log::debug!(
                    "Existing block {} wins fork choice, skipping synced block",
                    block.index
                );
            }
        } else {
            // No existing block, check parent
            if block.index > 0 {
                match MinerBlock::find_by_hash_multi(mgr, &block.previous_hash).await? {
                    Some(parent) if parent.is_canonical => {
                        block.save_to_active(mgr).await?;
                        saved_count += 1;
                        log::debug!(
                            "Saved synced block {} (index: {})",
                            &block.hash[..16],
                            block.index
                        );
                    }
                    Some(_) => {
                        log::warn!(
                            "Parent block {} is not canonical, skipping block {}",
                            &block.previous_hash[..16],
                            block.index
                        );
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

/// Next index to request after one range page.
///
/// `covered_to` is the server's `to_index`: the last index the page
/// included, whether or not every index in that window had a row.
/// Falling back to `from + row count` skips indexes when duplicate rows
/// make the page longer than the index window.
/// Last index a sync request must cover.
///
/// `row_count` is how many canonical rows the peer stored.
/// `chain_tip` is the highest index. Holes make the tip larger than
/// the row count; duplicate rows make the row count larger than the
/// tip. The request has to reach whichever is higher.
pub fn sync_fetch_end(row_count: u64, chain_tip: u64) -> u64 {
    row_count.max(chain_tip)
}

/// Outcome of one hash lookup. A timeout is not the same as a peer that
/// does not store the block: the next pass can ask again.
pub enum HashLookup {
    Block(MinerBlock),
    NotFound,
    Unavailable,
}

/// Request one block by hash.
///
/// The parent walk issues many of these, so a slow peer fails this one
/// request in a few seconds instead of holding the walk for a minute.
pub async fn request_block_by_hash(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    hash: &str,
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) -> Result<HashLookup> {
    use libp2p::multiaddr::Multiaddr;

    const PARENT_LOOKUP_TIMEOUT_SECS: u64 = 5;

    let ma: Multiaddr = peer_addr.parse()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Invalid peer address - missing PeerID");
    };

    let request = reqres::Request {
        path: "/data/miner_block/get".to_string(),
        data: Some(serde_json::json!({ "hash": hash })),
    };
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock
            .behaviour_mut()
            .reqres
            .send_request(&target_peer_id, request)
    };
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(PARENT_LOOKUP_TIMEOUT_SECS),
        wait_for_reqres_response(reqres_response_txs, request_id),
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            log::warn!("Failed to get block {hash}: {e}");
            return Ok(HashLookup::Unavailable);
        }
        Err(_) => {
            log::warn!("Block request for {hash} timed out");
            return Ok(HashLookup::Unavailable);
        }
    };
    if !response.ok {
        return Ok(HashLookup::NotFound);
    }
    let Some(data) = response.data else {
        return Ok(HashLookup::NotFound);
    };
    match serde_json::from_value::<MinerBlock>(data) {
        Ok(block) if block.hash == hash => Ok(HashLookup::Block(block)),
        Ok(_) => Ok(HashLookup::NotFound),
        Err(e) => {
            log::warn!("Failed to parse block {hash}: {e}");
            Ok(HashLookup::Unavailable)
        }
    }
}

pub fn next_range_index(from_index: u64, block_count: usize, covered_to: Option<u64>) -> u64 {
    match covered_to {
        Some(to) => to.saturating_add(1),
        None => from_index.saturating_add(block_count as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::{next_range_index, sync_fetch_end};

    #[test]
    fn duplicate_rows_do_not_skip_the_next_index() {
        // Page covered indexes 680..=728 and returned 55 rows because
        // five indexes were stored twice. Stepping by row count would
        // resume at 735 and drop 729..=734.
        assert_eq!(next_range_index(680, 55, Some(728)), 729);
    }

    #[test]
    fn missing_covered_to_still_advances_by_row_count() {
        assert_eq!(next_range_index(10, 4, None), 14);
    }

    #[test]
    fn fetch_covers_a_tip_above_the_row_count() {
        assert_eq!(sync_fetch_end(804, 984), 984);
        assert_eq!(sync_fetch_end(1087, 978), 1087);
    }
}
