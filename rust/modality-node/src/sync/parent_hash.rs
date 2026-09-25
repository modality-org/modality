//! Fetch the parent hash a parked run is waiting on.
//!
//! A gapped index range is not one chain. Each parked run names the
//! missing parent hash. This asks the peer for that hash, stores the
//! block off the canonical set, and repeats with that block's parent.
//! A peer that does not have the hash ends the walk for that run.

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::chain::fork_choice::{check_expected_target, RetargetParams, TargetCheck};
use crate::chain::reorg::{
    adopt_connected_extensions, missing_parent_hashes, select_best_stored_chain,
};
use crate::reqres;
use crate::sync::block_range::request_block_by_hash;

/// Ask `peer_addr` for parked parent hashes, then for each returned block's parent.
///
/// A hash no peer has is skipped for ten minutes so the walk can spend its
/// budget on the newest gap. Successes, not refusals, count toward the cap.
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
    const MAX_STORED_PARENTS: usize = 512;
    const MAX_ATTEMPTS: usize = 2048;

    let mut ended = HashSet::new();
    let mut stored_parents = 0usize;
    let mut missing = current_missing(datastore).await;
    for _ in 0..MAX_ATTEMPTS {
        if stored_parents >= MAX_STORED_PARENTS {
            break;
        }
        let Some(hash) = first_fetchable(&missing, &ended, peer_addr) else {
            break;
        };
        let hash = hash.clone();
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
            remember_absent(peer_addr, &hash);
            ended.insert(hash);
            continue;
        };
        if store_parent(datastore, &block).await {
            stored_parents += 1;
            missing = current_missing(datastore).await;
        } else {
            ended.insert(hash);
        }
    }
}

fn absent_cache() -> &'static StdMutex<HashMap<String, Instant>> {
    static CACHE: OnceLock<StdMutex<HashMap<String, Instant>>> = OnceLock::new();
    CACHE.get_or_init(|| StdMutex::new(HashMap::new()))
}

fn remember_absent(peer: &str, hash: &str) {
    let mut cache = absent_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    cache.insert(format!("{peer}\n{hash}"), Instant::now());
    if cache.len() > 8192 {
        cache.retain(|_, seen| seen.elapsed() < ABSENT_FOR);
    }
}

fn still_absent(peer: &str, hash: &str) -> bool {
    let cache = absent_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    cache
        .get(&format!("{peer}\n{hash}"))
        .is_some_and(|seen| seen.elapsed() < ABSENT_FOR)
}

const ABSENT_FOR: Duration = Duration::from_secs(600);

async fn current_missing(datastore: &Arc<Mutex<DatastoreManager>>) -> Vec<String> {
    let ds = datastore.lock().await;
    let Ok(blocks) = MinerBlock::find_all_blocks_multi(&ds).await else {
        return Vec::new();
    };
    missing_parent_hashes(&blocks)
}

/// Next parent to ask for. Hashes in `skip` were already tried this call,
/// and hashes peers recently lacked stay skipped.
pub(crate) fn first_fetchable<'a>(
    missing: &'a [String],
    skip: &HashSet<String>,
    peer: &str,
) -> Option<&'a String> {
    missing
        .iter()
        .find(|hash| !skip.contains(*hash) && !still_absent(peer, hash))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recently_missing_hash_is_skipped() {
        let missing = vec![
            "gone-parent-hash".to_string(),
            "live-parent-hash".to_string(),
        ];
        remember_absent("peer-a", "gone-parent-hash");
        let next = first_fetchable(&missing, &HashSet::new(), "peer-a");
        assert_eq!(next.map(String::as_str), Some("live-parent-hash"));
        let other_peer = first_fetchable(&missing, &HashSet::new(), "peer-b");
        assert_eq!(other_peer.map(String::as_str), Some("gone-parent-hash"));
    }
}
