//! Fetch the parent hash a parked run is waiting on.
//!
//! A gapped index range is not one chain. Each parked run names the
//! missing parent hash. This asks the peer for that hash, stores the
//! block off the canonical set, and repeats with that block's parent.
//! A peer that does not have the hash ends the walk for that run.

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::fork_choice::{check_expected_target, RetargetParams, TargetCheck};
use crate::chain::reorg::{
    adopt_connected_extensions, missing_parent_hashes, select_best_stored_chain,
};
use crate::reqres;
use crate::sync::block_range::request_block_by_hash;

/// Ask `peer_addr` for parked parent hashes, then for each returned block's parent.
pub async fn fetch_missing_parents(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) {
    let mut ended = HashSet::new();
    for _ in 0..64 {
        let Some(hash) = next_missing_hash(datastore, &ended).await else {
            return;
        };
        let shown = hash.len().min(16);
        log::info!("Requesting parent {} by hash", &hash[..shown]);
        let fetched =
            match request_block_by_hash(swarm, peer_addr, &hash, reqres_response_txs).await {
                Ok(block) => block,
                Err(e) => {
                    log::warn!("Failed to request parent {}: {e}", &hash[..shown]);
                    ended.insert(hash);
                    continue;
                }
            };
        let Some(block) = fetched.filter(|block| block.hash == hash) else {
            log::info!("Peer has no parent {}", &hash[..shown]);
            ended.insert(hash);
            continue;
        };
        if !store_parent(datastore, &block).await {
            ended.insert(hash);
        }
    }
}

async fn next_missing_hash(
    datastore: &Arc<Mutex<DatastoreManager>>,
    ended: &HashSet<String>,
) -> Option<String> {
    let ds = datastore.lock().await;
    let blocks = MinerBlock::find_all_blocks_multi(&ds).await.ok()?;
    missing_parent_hashes(&blocks)
        .into_iter()
        .find(|hash| !ended.contains(hash))
}

async fn store_parent(datastore: &Arc<Mutex<DatastoreManager>>, block: &MinerBlock) -> bool {
    let ds = datastore.lock().await;
    let stored = match MinerBlock::find_all_blocks_multi(&ds).await {
        Ok(blocks) => blocks,
        Err(e) => {
            log::warn!("Could not load blocks to store parent {}: {e}", block.hash);
            return false;
        }
    };
    let live: Vec<MinerBlock> = stored
        .into_iter()
        .filter(|block| !block.is_orphaned)
        .collect();
    let params = RetargetParams {
        blocks_per_epoch: ds.epoch_config().blocks_per_epoch,
        target_block_time_secs: ds.network_u64("target_block_time_secs").unwrap_or(60),
        initial_difficulty: ds
            .network_u64("initial_difficulty")
            .map(|value| value as u128),
    };
    if MinerBlock::find_by_hash_multi(&ds, &block.hash)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return false;
    }
    if check_expected_target(block, &live, params) == TargetCheck::Reject {
        log::warn!(
            "Parent {} at index {} rejected: target is not the difficulty this chain expects",
            &block.hash[..16.min(block.hash.len())],
            block.index
        );
        return false;
    }
    let mut parked = block.clone();
    parked.is_canonical = false;
    parked.is_orphaned = false;
    parked.orphan_reason = None;
    if let Err(e) = parked.save_to_active(&ds).await {
        log::warn!("Failed to store parent {}: {e}", block.hash);
        return false;
    }
    if let Err(e) = select_best_stored_chain(&ds).await {
        log::warn!(
            "Failed to score stored forks after parent {}: {e}",
            block.hash
        );
    }
    if let Err(e) = adopt_connected_extensions(&ds).await {
        log::warn!(
            "Failed to adopt a linked extension after parent {}: {e}",
            block.hash
        );
    }
    true
}
