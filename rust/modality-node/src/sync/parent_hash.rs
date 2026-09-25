//! Fetch the parent hash a parked run is waiting on.
//!
//! A peer tip is one chain: that hash, then its `previous_hash`, and so on
//! until the hash is already canonical here. Gaps under other stored blocks
//! are a different set and are not that chain.

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
use crate::sync::block_range::{request_block_by_hash, HashLookup};

/// Ask `peer_addr` for parent hashes.
///
/// When `peer_tip` is set, the next hash is always that block's
/// `previous_hash`. A timeout is not recorded as missing. A target mismatch
/// or a peer that does not have the parent stops this chain. Successes, not
/// refusals, count toward the cap.
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
    peer_tip: Option<&str>,
) {
    const MAX_STORED_PARENTS: usize = 512;
    const MAX_ATTEMPTS: usize = 2048;

    if let Some(tip) = peer_tip.filter(|tip| !tip.is_empty()) {
        follow_peer_chain(swarm, peer_addr, datastore, reqres_response_txs, tip).await;
        return;
    }

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
                Ok(lookup) => lookup,
                Err(e) => {
                    log::warn!("Failed to request parent {}: {e}", &hash[..shown]);
                    ended.insert(hash);
                    continue;
                }
            };
        let block = match fetched {
            HashLookup::Block(block) => block,
            HashLookup::NotFound => {
                log::info!("Peer has no parent {}", &hash[..shown]);
                remember_absent(peer_addr, &hash);
                ended.insert(hash);
                continue;
            }
            HashLookup::Unavailable => {
                log::info!("Parent {} was not answered in time", &hash[..shown]);
                ended.insert(hash);
                continue;
            }
        };
        match store_parent(datastore, &block).await {
            StoreParent::Stored => {
                stored_parents += 1;
                ended.insert(hash);
                missing = current_missing(datastore).await;
            }
            StoreParent::Rejected => {
                remember_rejected(&hash);
                ended.insert(hash);
            }
            StoreParent::AlreadyThere | StoreParent::Failed => {
                ended.insert(hash);
            }
        }
    }
}

/// Follow `start` through each block's `previous_hash`.
///
/// Blocks already stored are skipped locally, so a later sync resumes at the
/// first missing parent of this chain. A higher-index gap on disk is not
/// requested from here.
async fn follow_peer_chain(
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
    start: &str,
) {
    const MAX_STORED_PARENTS: usize = 512;
    const MAX_ATTEMPTS: usize = 2048;

    let mut cursor = start.to_string();
    let mut seen = HashSet::new();
    let mut stored_parents = 0usize;
    for _ in 0..MAX_ATTEMPTS {
        if stored_parents >= MAX_STORED_PARENTS || !seen.insert(cursor.clone()) {
            break;
        }
        if still_rejected(&cursor) || still_absent(peer_addr, &cursor) {
            break;
        }
        if let Some(existing) = stored_by_hash(datastore, &cursor).await {
            if existing.is_canonical && !existing.is_orphaned {
                let shown = cursor.len().min(16);
                log::info!("Peer chain met local canonical block {}", &cursor[..shown]);
                break;
            }
            match parent_to_follow(existing.index, &existing.previous_hash) {
                Some(parent) => {
                    cursor = parent.to_string();
                    continue;
                }
                None => break,
            }
        }
        let shown = cursor.len().min(16);
        log::info!("Following parent {} by hash", &cursor[..shown]);
        let fetched =
            match request_block_by_hash(swarm, peer_addr, &cursor, reqres_response_txs).await {
                Ok(lookup) => lookup,
                Err(e) => {
                    log::warn!("Failed to follow parent {}: {e}", &cursor[..shown]);
                    break;
                }
            };
        let block = match fetched {
            HashLookup::Block(block) if block.hash == cursor => block,
            HashLookup::Block(_) | HashLookup::NotFound => {
                log::info!("Peer has no parent {}", &cursor[..shown]);
                remember_absent(peer_addr, &cursor);
                break;
            }
            HashLookup::Unavailable => {
                log::info!("Parent {} was not answered in time", &cursor[..shown]);
                break;
            }
        };
        match store_parent(datastore, &block).await {
            StoreParent::Rejected => {
                remember_rejected(&cursor);
                break;
            }
            StoreParent::Failed => break,
            StoreParent::Stored => stored_parents += 1,
            StoreParent::AlreadyThere => {}
        }
        match parent_to_follow(block.index, &block.previous_hash) {
            Some(parent) => cursor = parent.to_string(),
            None => break,
        }
    }
}

async fn stored_by_hash(
    datastore: &Arc<Mutex<DatastoreManager>>,
    hash: &str,
) -> Option<MinerBlock> {
    let ds = datastore.lock().await;
    MinerBlock::find_by_hash_multi(&ds, hash)
        .await
        .ok()
        .flatten()
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

fn remember_rejected(hash: &str) {
    let mut cache = absent_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    cache.insert(format!("reject\n{hash}"), Instant::now());
}

fn still_rejected(hash: &str) -> bool {
    let cache = absent_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    cache
        .get(&format!("reject\n{hash}"))
        .is_some_and(|seen| seen.elapsed() < ABSENT_FOR)
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

/// The next hash on this chain is `previous_hash`.
///
/// A higher index stored from another fork is not an input. Index 0 and an
/// empty parent end the walk.
pub(crate) fn parent_to_follow(index: u64, previous_hash: &str) -> Option<&str> {
    if index == 0 || previous_hash.is_empty() {
        None
    } else {
        Some(previous_hash)
    }
}

/// Next parent to ask for. Hashes in `skip` were already tried this call.
/// A peer that recently lacked a hash stays skipped for that peer, and a
/// hash whose target does not match this chain stays skipped for every peer.
pub(crate) fn first_fetchable<'a>(
    missing: &'a [String],
    skip: &HashSet<String>,
    peer: &str,
) -> Option<&'a String> {
    missing
        .iter()
        .find(|hash| !skip.contains(*hash) && !still_absent(peer, hash) && !still_rejected(hash))
}

enum StoreParent {
    Stored,
    AlreadyThere,
    Rejected,
    Failed,
}

async fn store_parent(datastore: &Arc<Mutex<DatastoreManager>>, block: &MinerBlock) -> StoreParent {
    let ds = datastore.lock().await;
    let stored = match MinerBlock::find_all_blocks_multi(&ds).await {
        Ok(blocks) => blocks,
        Err(e) => {
            log::warn!("Could not load blocks to store parent {}: {e}", block.hash);
            return StoreParent::Failed;
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
        return StoreParent::AlreadyThere;
    }
    if check_expected_target(block, &live, params) == TargetCheck::Reject {
        log::warn!(
            "Parent {} at index {} rejected: target is not the difficulty this chain expects",
            &block.hash[..16.min(block.hash.len())],
            block.index
        );
        return StoreParent::Rejected;
    }
    let mut parked = block.clone();
    parked.is_canonical = false;
    parked.is_orphaned = false;
    parked.orphan_reason = None;
    if let Err(e) = parked.save_to_active(&ds).await {
        log::warn!("Failed to store parent {}: {e}", block.hash);
        return StoreParent::Failed;
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
    StoreParent::Stored
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

    #[test]
    fn a_rejected_target_is_skipped_for_every_peer() {
        let missing = vec![
            "wrong-target-parent".to_string(),
            "canonical-parent".to_string(),
        ];
        remember_rejected("wrong-target-parent");
        let next = first_fetchable(&missing, &HashSet::new(), "peer-a");
        assert_eq!(next.map(String::as_str), Some("canonical-parent"));
    }

    #[test]
    fn the_next_fetch_is_that_blocks_parent_hash() {
        assert_eq!(
            parent_to_follow(1113, "00173270-parent"),
            Some("00173270-parent")
        );
        assert_eq!(parent_to_follow(1113, ""), None);
        assert_eq!(parent_to_follow(0, "genesis-parent"), None);
    }
}
