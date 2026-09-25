//! Chain reorganization utilities.
//!
//! This module provides functions for chain reorganization including
//! orphaning blocks and cascade orphaning.

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::{BTreeMap, HashSet};

/// Result of an orphaning operation
#[derive(Debug, Clone)]
pub struct OrphanResult {
    /// Number of blocks that were orphaned
    pub orphaned_count: usize,
    /// Hashes of orphaned blocks
    pub orphaned_hashes: Vec<String>,
    /// Index where orphaning started
    pub start_index: u64,
}

/// Orphan all canonical blocks after a given index.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `after_index` - Orphan blocks with index > after_index
/// * `reason` - Reason for orphaning (stored in block metadata)
///
/// # Returns
/// Result containing orphan statistics
pub async fn orphan_blocks_after(
    mgr: &DatastoreManager,
    after_index: u64,
    reason: &str,
) -> Result<OrphanResult> {
    let all_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;

    let mut orphaned_count = 0;
    let mut orphaned_hashes = Vec::new();

    for block in &all_blocks {
        if block.index > after_index && block.is_canonical && !block.is_orphaned {
            let mut orphaned = block.clone();
            orphaned.mark_as_orphaned(reason.to_string(), None);

            if let Err(e) = orphaned.save_to_active(mgr).await {
                log::error!(
                    "Failed to orphan block {} at index {}: {}",
                    &block.hash[..16],
                    block.index,
                    e
                );
            } else {
                log::info!(
                    "   Orphaned block {} at index {}",
                    &block.hash[..16],
                    block.index
                );
                orphaned_hashes.push(block.hash.clone());
                orphaned_count += 1;
            }
        }
    }

    Ok(OrphanResult {
        orphaned_count,
        orphaned_hashes,
        start_index: after_index + 1,
    })
}

/// Cascade orphan blocks that build on an orphaned block.
///
/// When a block is orphaned, all blocks that reference it (directly or indirectly)
/// as their parent must also be orphaned.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `orphaned_block_hash` - Hash of the initially orphaned block
/// * `orphaned_block_index` - Index of the initially orphaned block
/// * `reason_prefix` - Prefix for the orphan reason
///
/// # Returns
/// Number of cascade-orphaned blocks
pub async fn cascade_orphan(
    mgr: &DatastoreManager,
    orphaned_block_hash: &str,
    orphaned_block_index: u64,
    reason_prefix: &str,
) -> Result<usize> {
    let all_canonical = MinerBlock::find_all_canonical_multi(mgr).await?;

    // Find blocks that might need cascade orphaning
    let mut blocks_to_check: Vec<_> = all_canonical
        .iter()
        .filter(|b| b.index > orphaned_block_index && b.is_canonical && !b.is_orphaned)
        .collect();

    blocks_to_check.sort_by_key(|b| b.index);

    // Track which hashes have been orphaned
    let mut orphaned_hashes = HashSet::new();
    orphaned_hashes.insert(orphaned_block_hash.to_string());

    let mut cascade_count = 0;

    for block in blocks_to_check {
        // If this block's parent was orphaned, orphan this block too
        if orphaned_hashes.contains(&block.previous_hash) {
            log::info!(
                "   Cascade orphaning block {} at index {} (built on orphaned chain)",
                &block.hash[..16],
                block.index
            );

            let mut cascade_orphaned = block.clone();
            cascade_orphaned.mark_as_orphaned(
                format!(
                    "{}: built on orphaned block {} at index {}",
                    reason_prefix,
                    &orphaned_block_hash[..16.min(orphaned_block_hash.len())],
                    orphaned_block_index
                ),
                None,
            );
            cascade_orphaned.save_to_active(mgr).await?;

            orphaned_hashes.insert(block.hash.clone());
            cascade_count += 1;
        }
    }

    if cascade_count > 0 {
        log::warn!(
            "⚠️  Cascade orphaned {} blocks built on orphaned block {}",
            cascade_count,
            orphaned_block_index
        );
    }

    Ok(cascade_count)
}

/// Orphan a single block and cascade to dependents.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `block` - The block to orphan
/// * `reason` - Reason for orphaning
/// * `competing_hash` - Optional hash of the competing block that replaced this one
///
/// # Returns
/// Total number of blocks orphaned (including cascade)
pub async fn orphan_block_with_cascade(
    mgr: &DatastoreManager,
    block: &MinerBlock,
    reason: &str,
    competing_hash: Option<String>,
) -> Result<usize> {
    let block_hash = block.hash.clone();
    let block_index = block.index;

    // Orphan the primary block
    let mut orphaned = block.clone();
    orphaned.mark_as_orphaned(reason.to_string(), competing_hash);
    orphaned.save_to_active(mgr).await?;

    log::info!(
        "Orphaned block {} at index {}",
        &block_hash[..16],
        block_index
    );

    // Cascade to dependent blocks
    let cascade_count =
        cascade_orphan(mgr, &block_hash, block_index, "Cascade from fork choice").await?;

    Ok(1 + cascade_count)
}

/// Find the common ancestor between local blocks and a set of remote block hashes.
///
/// # Arguments
/// * `local_blocks` - Local canonical blocks
/// * `remote_hashes` - Set of remote block hashes
///
/// # Returns
/// Index of the highest common block, or None if no common ancestor
pub fn find_common_ancestor_by_hash(
    local_blocks: &[MinerBlock],
    remote_hashes: &HashSet<String>,
) -> Option<u64> {
    // Sort blocks by index descending to find highest common ancestor first
    let mut sorted_blocks: Vec<_> = local_blocks.iter().collect();
    sorted_blocks.sort_by_key(|block| std::cmp::Reverse(block.index));

    for block in sorted_blocks {
        if remote_hashes.contains(&block.hash) {
            return Some(block.index);
        }
    }

    None
}

/// Indexes under the accepted tip that the parent walk could not cross.
///
/// `to_index` is the missing parent of the linked suffix. `expected_hash` is
/// the hash that parent must have. `from_index` is the first index after the
/// highest stored block below that suffix, so one request covers the hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentGap {
    pub from_index: u64,
    pub to_index: u64,
    pub expected_hash: String,
}

pub fn tip_parent_gap(blocks: &[MinerBlock]) -> Option<ParentGap> {
    if blocks.is_empty() {
        return None;
    }

    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let tip_index = *by_index.keys().next_back()?;
    let mut best_len = 0u64;
    let mut best_start = tip_index;
    let mut expected_hash = String::new();
    let mut found = false;

    for tip in &by_index[&tip_index] {
        let mut len = 1u64;
        let mut current = *tip;
        while current.index > 0 {
            let Some(parents) = by_index.get(&(current.index - 1)) else {
                break;
            };
            let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
                break;
            };
            current = parent;
            len += 1;
        }
        if !found || len > best_len {
            found = true;
            best_len = len;
            best_start = current.index;
            expected_hash = current.previous_hash.clone();
        }
    }

    if !found || best_start == 0 {
        return None;
    }

    let to_index = best_start - 1;
    let from_index = by_index
        .range(..best_start)
        .next_back()
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    Some(ParentGap {
        from_index: from_index.min(to_index),
        to_index,
        expected_hash,
    })
}

/// Blocks from `blocks` that end at `expected_hash` on `to_index` and follow
/// `previous_hash` downward. The tip above this gap is not included.
pub fn linking_parents(
    blocks: &[MinerBlock],
    to_index: u64,
    expected_hash: &str,
) -> Vec<MinerBlock> {
    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let Some(end) = by_index
        .get(&to_index)
        .and_then(|at| at.iter().find(|b| b.hash == expected_hash))
    else {
        return Vec::new();
    };

    let mut chain = vec![(*end).clone()];
    let mut current = *end;
    while current.index > 0 {
        let Some(parents) = by_index.get(&(current.index - 1)) else {
            break;
        };
        let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
            break;
        };
        chain.push((*parent).clone());
        current = parent;
    }
    chain.reverse();
    chain
}

/// Collapse duplicate indexes onto the parent-linked chain.
///
/// A peer can list two canonical blocks at the same index. The returned
/// chain has to start at the earliest index in the batch. A higher run
/// that does not link back to that start is not a chain to adopt.
pub fn select_linked_chain(blocks: &[MinerBlock]) -> Result<Vec<MinerBlock>> {
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let tip_index = *by_index.keys().next_back().expect("indexes non-empty");
    let mut best: Vec<MinerBlock> = Vec::new();
    for tip in &by_index[&tip_index] {
        let mut chain = vec![(*tip).clone()];
        let mut current = *tip;
        while current.index > 0 {
            let Some(parents) = by_index.get(&(current.index - 1)) else {
                break;
            };
            let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
                break;
            };
            chain.push((*parent).clone());
            current = parent;
        }
        if chain.len() > best.len() {
            best = chain;
        }
    }

    if best.is_empty() {
        anyhow::bail!("Invalid chain: duplicate indexes do not form one linked chain");
    }
    let start = best.last().expect("best non-empty").index;
    let earliest = *by_index.keys().next().expect("indexes non-empty");
    if start != earliest {
        anyhow::bail!(
            "disconnected suffix {start}..={tip_index} is not a chain (batch starts at {earliest})"
        );
    }
    best.reverse();
    Ok(best)
}

/// One parent-linked run taken from a peer reply that may contain holes.
///
/// `missing_parent` is the `previous_hash` of the lowest block. That hash
/// is the next block to fetch. The run stays off the canonical chain until
/// a walk through stored blocks reaches a canonical ancestor.
#[derive(Debug, Clone)]
pub struct LinkedRun {
    pub blocks: Vec<MinerBlock>,
    pub missing_parent: String,
}

/// Split a batch into maximal parent-linked runs.
///
/// A hole starts a new run. The higher run is kept, named by the parent
/// hash it is waiting on.
pub fn split_linked_runs(blocks: &[MinerBlock]) -> Vec<LinkedRun> {
    let mut by_hash: BTreeMap<&str, &MinerBlock> = BTreeMap::new();
    for block in blocks {
        by_hash.entry(block.hash.as_str()).or_insert(block);
    }
    let mut children: BTreeMap<&str, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        if let Some(parent) = by_hash.get(block.previous_hash.as_str()) {
            if parent.index + 1 == block.index {
                children
                    .entry(parent.hash.as_str())
                    .or_default()
                    .push(block);
            }
        }
    }
    let mut seen_bottoms = HashSet::new();
    let mut runs = Vec::new();
    for bottom in blocks {
        let parent_in_batch = by_hash
            .get(bottom.previous_hash.as_str())
            .is_some_and(|parent| parent.index + 1 == bottom.index);
        if parent_in_batch || !seen_bottoms.insert(bottom.hash.as_str()) {
            continue;
        }
        let mut stack = vec![vec![bottom]];
        while let Some(path) = stack.pop() {
            let tip = path[path.len() - 1];
            let kids = children.get(tip.hash.as_str()).cloned().unwrap_or_default();
            if kids.is_empty() {
                runs.push(LinkedRun {
                    missing_parent: path[0].previous_hash.clone(),
                    blocks: path.into_iter().cloned().collect(),
                });
            } else {
                for kid in kids {
                    let mut next = path.clone();
                    next.push(kid);
                    stack.push(next);
                }
            }
        }
    }
    runs
}

/// Parent hashes of runs whose parent block is not stored at the previous index.
///
/// A contiguous chain that starts at index 0 has nothing to fetch.
pub fn missing_parent_hashes(blocks: &[MinerBlock]) -> Vec<String> {
    let live: Vec<&MinerBlock> = blocks.iter().filter(|block| !block.is_orphaned).collect();
    let owned: Vec<MinerBlock> = live.iter().map(|block| (*block).clone()).collect();
    let mut ranked: Vec<(u64, String)> = Vec::new();
    for run in split_linked_runs(&owned) {
        let Some(bottom) = run.blocks.first() else {
            continue;
        };
        if bottom.index == 0 {
            continue;
        }
        let present = live
            .iter()
            .any(|block| block.hash == run.missing_parent && block.index + 1 == bottom.index);
        if present || ranked.iter().any(|(_, hash)| hash == &run.missing_parent) {
            continue;
        }
        ranked.push((bottom.index, run.missing_parent));
    }
    // Highest index first among gaps this node already has. A peer tip is
    // followed by previous_hash and does not use this order.
    ranked.sort_by(|left, right| right.0.cmp(&left.0));
    ranked.into_iter().map(|(_, hash)| hash).collect()
}

/// Blocks of `run` that sit strictly above a local canonical block.
///
/// The walk has to reach a canonical ancestor. A run whose parent is only
/// another parked block is not an extension yet.
pub fn extension_from_local(run: &LinkedRun, stored: &[MinerBlock]) -> Option<Vec<MinerBlock>> {
    let bottom = run.blocks.first()?;
    let local_canonical = |hash: &str| {
        stored
            .iter()
            .any(|block| block.hash == hash && block.is_canonical && !block.is_orphaned)
    };
    if let Some(pos) = run
        .blocks
        .iter()
        .rposition(|block| local_canonical(&block.hash))
    {
        if pos + 1 >= run.blocks.len() {
            return None;
        }
        return Some(run.blocks[pos + 1..].to_vec());
    }
    let links = stored.iter().any(|block| {
        block.is_canonical
            && !block.is_orphaned
            && block.index + 1 == bottom.index
            && block.hash == bottom.previous_hash
    });
    if links {
        Some(run.blocks.clone())
    } else {
        None
    }
}

async fn save_parked_block(mgr: &DatastoreManager, block: &MinerBlock) -> Result<bool> {
    if let Ok(Some(_)) = MinerBlock::find_by_hash_multi(mgr, &block.hash).await {
        return Ok(false);
    }
    let mut parked = block.clone();
    parked.is_canonical = false;
    parked.is_orphaned = false;
    parked.orphan_reason = None;
    parked.save_to_active(mgr).await?;
    Ok(true)
}

/// Save every block in the reply off the canonical set, then adopt a run
/// only when it links to a local canonical block and wins verified work.
pub async fn store_peer_batch(mgr: &DatastoreManager, peer_blocks: &[MinerBlock]) -> Result<usize> {
    if peer_blocks.is_empty() {
        return Ok(0);
    }
    let stored = MinerBlock::find_all_blocks_multi(mgr).await?;
    let live: Vec<MinerBlock> = stored
        .into_iter()
        .filter(|block| !block.is_orphaned)
        .collect();
    for run in split_linked_runs(peer_blocks) {
        if extension_from_local(&run, &live).is_none() {
            let bottom = &run.blocks[0];
            let tip = &run.blocks[run.blocks.len() - 1];
            let parent = &run.missing_parent;
            log::info!(
                "Parked blocks {}..={} until parent {} arrives",
                bottom.index,
                tip.index,
                &parent[..16.min(parent.len())]
            );
        }
    }
    for block in peer_blocks {
        save_parked_block(mgr, block).await?;
    }
    let switched = select_best_stored_chain(mgr).await?;
    let adopted = adopt_connected_extensions(mgr).await?;
    Ok(adopted + usize::from(switched))
}

/// Promote parked runs whose parent walk now reaches the canonical chain
/// and whose verified work above that ancestor wins.
pub async fn adopt_connected_extensions(mgr: &DatastoreManager) -> Result<usize> {
    use crate::chain::fork_choice::{decide_adoption, Adoption};
    use modality_datastore::models::miner::MinerCheckpoint;

    let stored = MinerBlock::find_all_blocks_multi(mgr).await?;
    let live: Vec<MinerBlock> = stored
        .into_iter()
        .filter(|block| !block.is_orphaned)
        .collect();
    let extensions: Vec<Vec<MinerBlock>> = split_linked_runs(&live)
        .iter()
        .filter_map(|run| extension_from_local(run, &live))
        .filter(|extension| {
            extension.iter().any(|block| {
                live.iter()
                    .any(|stored| stored.hash == block.hash && !stored.is_canonical)
            })
        })
        .collect();

    let mut adopted = 0usize;
    for extension in extensions {
        let local_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
        if extension.iter().all(|block| {
            local_blocks
                .iter()
                .any(|canonical| canonical.hash == block.hash)
        }) {
            continue;
        }
        let floor = MinerCheckpoint::find_latest_multi(mgr)
            .await
            .ok()
            .flatten()
            .map(|checkpoint| checkpoint.last_block_index);
        let blocks_per_epoch = mgr.epoch_config().blocks_per_epoch;
        let safe = crate::chain::fork_choice::nomination_safe_prefix(
            &local_blocks,
            &extension,
            blocks_per_epoch,
        );
        if safe.is_empty() {
            let bottom = extension[0].index;
            let tip = extension[extension.len() - 1].index;
            log::debug!("Keeping parked extension {bottom}..={tip}: nomination epoch");
            continue;
        }
        match decide_adoption(&local_blocks, &safe, floor, blocks_per_epoch) {
            Adoption::Refuse { reason } => {
                let bottom = safe[0].index;
                let tip = safe[safe.len() - 1].index;
                log::debug!("Keeping parked extension {bottom}..={tip}: {reason}");
            }
            Adoption::Adopt {
                ancestor_index,
                reason,
            } => {
                log::info!("Adopting linked extension: {reason}");
                if safe.first().is_none_or(|block| block.index != 0) {
                    for local in &local_blocks {
                        if local.index > ancestor_index {
                            let competing_hash = safe
                                .iter()
                                .find(|block| block.index == local.index)
                                .map(|block| block.hash.clone());
                            let mut orphaned = local.clone();
                            orphaned.mark_as_orphaned(reason.clone(), competing_hash);
                            orphaned.save_to_active(mgr).await?;
                        }
                    }
                }
                for block in &safe {
                    let mut stored = block.clone();
                    stored.is_canonical = true;
                    stored.is_orphaned = false;
                    stored.orphan_reason = None;
                    stored.save_to_active(mgr).await?;
                }
                adopted += safe.len();
            }
        }
    }
    Ok(adopted)
}

/// Prepare blocks for adoption by validating chain continuity.
///
/// # Arguments
/// * `blocks` - Blocks to validate (should be sorted by index)
///
/// # Returns
/// Ok(()) if valid, Err with reason if invalid
pub fn validate_block_chain(blocks: &[MinerBlock]) -> Result<()> {
    if blocks.is_empty() {
        return Ok(());
    }

    for i in 1..blocks.len() {
        // Check consecutive indices
        if blocks[i].index != blocks[i - 1].index + 1 {
            anyhow::bail!(
                "Blocks not consecutive: gap between {} and {}",
                blocks[i - 1].index,
                blocks[i].index
            );
        }

        // Check hash linkage
        if blocks[i].previous_hash != blocks[i - 1].hash {
            anyhow::bail!(
                "Invalid chain: block {} prev_hash doesn't match block {} hash",
                blocks[i].index,
                blocks[i - 1].index
            );
        }
    }

    Ok(())
}

/// Make the heaviest stored chain canonical.
///
/// Canonical blocks and competing blocks (stored, not orphaned) are both
/// candidates. The loser stays stored so a later heavier extension can
/// win without another peer sending the whole suffix. A switch that would
/// drop a sequencer checkpoint, or a nomination epoch the current chain
/// already has, is refused.
pub async fn select_best_stored_chain(mgr: &DatastoreManager) -> Result<bool> {
    use crate::chain::fork_choice::{
        check_expected_target, choose_verified, nomination_epoch_complete, score_canonical_chain,
        ForkChoiceResult, RetargetParams, TargetCheck,
    };
    use modality_datastore::models::miner::MinerCheckpoint;

    let all = MinerBlock::find_all_blocks_multi(mgr).await?;
    let params = RetargetParams {
        blocks_per_epoch: mgr.epoch_config().blocks_per_epoch,
        target_block_time_secs: mgr.network_u64("target_block_time_secs").unwrap_or(60),
        initial_difficulty: mgr
            .network_u64("initial_difficulty")
            .map(|value| value as u128),
    };
    let live: Vec<MinerBlock> = all.into_iter().filter(|block| !block.is_orphaned).collect();
    let eligible: Vec<MinerBlock> = live
        .iter()
        .filter(|block| check_expected_target(block, &live, params) != TargetCheck::Reject)
        .cloned()
        .collect();
    let winner = MinerBlock::verified_spine(&eligible);
    if winner.is_empty() {
        return Ok(false);
    }
    let winner_hashes: HashSet<String> = winner.iter().map(|block| block.hash.clone()).collect();
    let canonical: Vec<MinerBlock> = live
        .iter()
        .filter(|block| block.is_canonical)
        .cloned()
        .collect();
    let floor = MinerCheckpoint::find_latest_multi(mgr)
        .await
        .ok()
        .flatten()
        .map(|checkpoint| checkpoint.last_block_index);
    let spine_hashes: HashSet<String> = MinerBlock::verified_spine(&canonical)
        .into_iter()
        .map(|block| block.hash)
        .collect();
    let mut demoted_off_spine = false;
    for block in &canonical {
        if spine_hashes.contains(&block.hash) {
            continue;
        }
        if floor.is_some_and(|floor| block.index <= floor) {
            continue;
        }
        let mut competing = block.clone();
        competing.is_canonical = false;
        competing.is_orphaned = false;
        competing.orphan_reason = Some("Off the verified spine".to_string());
        competing.save_to_active(mgr).await?;
        demoted_off_spine = true;
    }
    let canonical: Vec<MinerBlock> = if demoted_off_spine {
        MinerBlock::find_all_canonical_multi(mgr).await?
    } else {
        canonical
    };
    let canonical_hashes: HashSet<String> =
        canonical.iter().map(|block| block.hash.clone()).collect();
    if winner_hashes == canonical_hashes {
        return Ok(demoted_off_spine);
    }

    if let Some(floor) = floor {
        if canonical
            .iter()
            .any(|block| block.index <= floor && !winner_hashes.contains(&block.hash))
        {
            log::warn!("Refusing to switch off the sequenced prefix through {floor}");
            return Ok(demoted_off_spine);
        }
    }

    let blocks_per_epoch = params.blocks_per_epoch;
    let local_epoch = MinerBlock::verified_spine(&canonical)
        .last()
        .map(|block| block.epoch)
        .unwrap_or(0);
    let winner_epoch = winner.last().map(|block| block.epoch).unwrap_or(0);
    if nomination_epoch_complete(&canonical, blocks_per_epoch, local_epoch)
        && !nomination_epoch_complete(&winner, blocks_per_epoch, winner_epoch)
    {
        log::warn!("Refusing a fork that drops the nomination epoch");
        return Ok(demoted_off_spine);
    }

    let local_score = score_canonical_chain(&canonical);
    let winner_score = score_canonical_chain(&winner);
    let decision = choose_verified(
        local_score.work,
        local_score.tip,
        &local_score.tip_hash,
        winner_score.work,
        winner_score.tip,
        &winner_score.tip_hash,
    );
    let fills_parents = canonical_hashes.is_subset(&winner_hashes);
    if decision.result != ForkChoiceResult::AdoptRemote && !fills_parents {
        return Ok(demoted_off_spine);
    }

    for block in &live {
        let on_winner = winner_hashes.contains(&block.hash);
        if on_winner && !block.is_canonical {
            let mut promoted = block.clone();
            promoted.is_canonical = true;
            promoted.is_orphaned = false;
            promoted.orphan_reason = None;
            promoted.save_to_active(mgr).await?;
        } else if block.is_canonical && !on_winner {
            let mut competing = block.clone();
            competing.is_canonical = false;
            competing.is_orphaned = false;
            competing.orphan_reason = Some("Competing fork".to_string());
            competing.save_to_active(mgr).await?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_block(index: u64, prev_hash: &str) -> MinerBlock {
        MinerBlock::new_canonical(
            format!("hash_{}", index),
            index,
            0,
            1234567890 + index as i64,
            prev_hash.to_string(),
            format!("data_{}", index),
            12345,
            1000,
            "peer_id".to_string(),
            1,
        )
    }

    #[test]
    fn tip_parent_gap_is_the_hole_under_the_linked_suffix() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let gap = tip_parent_gap(&blocks).expect("gap");
        assert_eq!(gap.from_index, 2);
        assert_eq!(gap.to_index, 3);
        assert_eq!(gap.expected_hash, "hash_3");
    }

    #[test]
    fn wrong_block_at_the_parent_index_requests_that_index() {
        let mut decoy = make_test_block(3, "hash_2");
        decoy.hash = "other_3".to_string();
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            decoy,
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let gap = tip_parent_gap(&blocks).expect("gap");
        assert_eq!(gap.from_index, 3);
        assert_eq!(gap.to_index, 3);
        assert_eq!(gap.expected_hash, "hash_3");
    }

    #[test]
    fn contiguous_tip_has_no_parent_gap() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        assert!(tip_parent_gap(&blocks).is_none());
    }

    #[test]
    fn linking_parents_follow_the_expected_hash_only() {
        let mut decoy = make_test_block(3, "hash_2");
        decoy.hash = "other_3".to_string();
        let fetched = vec![
            make_test_block(2, "hash_1"),
            decoy,
            make_test_block(3, "hash_2"),
        ];
        let linked = linking_parents(&fetched, 3, "hash_3");
        assert_eq!(
            linked.iter().map(|b| b.hash.as_str()).collect::<Vec<_>>(),
            vec!["hash_2", "hash_3"]
        );
    }

    #[test]
    fn test_validate_block_chain_valid() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];

        assert!(validate_block_chain(&blocks).is_ok());
    }

    #[test]
    fn duplicate_index_keeps_the_linked_block() {
        let mut fork = make_test_block(1, "hash_0");
        fork.hash = "other_1".to_string();
        let blocks = vec![
            make_test_block(0, "genesis"),
            fork,
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        let linked = select_linked_chain(&blocks).unwrap();
        assert_eq!(linked.len(), 3);
        assert_eq!(linked[1].hash, "hash_1");
        assert!(validate_block_chain(&linked).is_ok());
    }

    #[test]
    fn real_index_gap_is_not_adoptable() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let err = select_linked_chain(&blocks).unwrap_err();
        assert!(err.to_string().contains("disconnected suffix"));
    }

    #[test]
    fn test_validate_block_chain_gap() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(2, "hash_1"), // Gap - missing index 1
        ];

        assert!(validate_block_chain(&blocks).is_err());
    }

    #[test]
    fn test_validate_block_chain_bad_link() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "wrong_hash"), // Bad link
        ];

        assert!(validate_block_chain(&blocks).is_err());
    }

    #[tokio::test]
    async fn a_heavier_competing_fork_becomes_canonical() {
        let datastore = modality_datastore::DatastoreManager::create_in_memory().unwrap();
        for block in [
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ] {
            block.save_to_active(&datastore).await.unwrap();
        }
        let mut alt = make_test_block(1, "hash_0");
        alt.hash = "alt_1".to_string();
        alt.is_canonical = false;
        alt.actualized_difficulty = "1000".to_string();
        alt.save_to_active(&datastore).await.unwrap();
        let mut heavy = make_test_block(2, "alt_1");
        heavy.hash = "alt_2".to_string();
        heavy.previous_hash = "alt_1".to_string();
        heavy.is_canonical = false;
        heavy.actualized_difficulty = "5000".to_string();
        heavy.target_difficulty = "1000".to_string();
        heavy.save_to_active(&datastore).await.unwrap();
        let mut extension = make_test_block(3, "alt_2");
        extension.hash = "alt_3".to_string();
        extension.previous_hash = "alt_2".to_string();
        extension.is_canonical = false;
        extension.actualized_difficulty = "5000".to_string();
        extension.target_difficulty = "1000".to_string();
        extension.save_to_active(&datastore).await.unwrap();

        assert!(select_best_stored_chain(&datastore).await.unwrap());
        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<_> = canonical.iter().map(|block| block.hash.as_str()).collect();
        assert!(hashes.contains(&"alt_1"));
        assert!(hashes.contains(&"alt_3"));
        assert!(!hashes.contains(&"hash_1"));
        let parked = MinerBlock::find_by_hash_multi(&datastore, "hash_1")
            .await
            .unwrap()
            .unwrap();
        assert!(!parked.is_canonical);
        assert!(!parked.is_orphaned);
    }

    #[tokio::test]
    async fn a_parked_child_links_when_its_parent_is_stored() {
        let datastore = modality_datastore::DatastoreManager::create_in_memory().unwrap();
        make_test_block(0, "genesis")
            .save_to_active(&datastore)
            .await
            .unwrap();
        let mut child = make_test_block(1, "hash_0");
        child.is_canonical = false;
        child.save_to_active(&datastore).await.unwrap();

        assert!(select_best_stored_chain(&datastore).await.unwrap());
        let stored = MinerBlock::find_by_hash_multi(&datastore, "hash_1")
            .await
            .unwrap()
            .unwrap();
        assert!(stored.is_canonical);
    }

    #[test]
    fn a_gapped_reply_keeps_the_disconnected_suffix() {
        let low = make_test_block(1, "hash_0");
        let high = make_test_block(4, "absent");
        let next = make_test_block(5, "hash_4");
        let runs = split_linked_runs(&[low, high, next]);
        assert_eq!(runs.len(), 2);
        let parked = runs
            .iter()
            .find(|run| run.blocks[0].index == 4)
            .expect("high run");
        assert_eq!(parked.blocks.len(), 2);
        assert_eq!(parked.missing_parent, "absent");
    }

    #[test]
    fn the_newest_gap_is_fetched_before_an_older_one() {
        let old = make_test_block(4, "old_parent");
        let new = make_test_block(900, "new_parent");
        let hashes = missing_parent_hashes(&[old, new]);
        assert_eq!(
            hashes,
            vec!["new_parent".to_string(), "old_parent".to_string()]
        );
    }

    #[test]
    fn a_contiguous_chain_has_no_missing_parent() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        assert!(missing_parent_hashes(&blocks).is_empty());
    }

    #[tokio::test]
    async fn a_disconnected_suffix_stays_parked_until_its_parent_arrives() {
        let datastore = modality_datastore::DatastoreManager::create_in_memory().unwrap();
        for block in [
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ] {
            block.save_to_active(&datastore).await.unwrap();
        }
        let high = make_test_block(4, "absent");
        let next = make_test_block(5, "hash_4");
        store_peer_batch(&datastore, &[high, next]).await.unwrap();

        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<_> = canonical.iter().map(|block| block.hash.as_str()).collect();
        assert!(hashes.contains(&"hash_2"));
        assert!(!hashes.contains(&"hash_4"));
        let parked = MinerBlock::find_by_hash_multi(&datastore, "hash_4")
            .await
            .unwrap()
            .unwrap();
        assert!(!parked.is_canonical);
        assert!(!parked.is_orphaned);
        assert_eq!(
            missing_parent_hashes(&MinerBlock::find_all_blocks_multi(&datastore).await.unwrap()),
            vec!["absent".to_string()]
        );

        let mut parent = make_test_block(3, "hash_2");
        parent.hash = "absent".to_string();
        parent.is_canonical = false;
        parent.save_to_active(&datastore).await.unwrap();
        select_best_stored_chain(&datastore).await.unwrap();

        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<_> = canonical.iter().map(|block| block.hash.as_str()).collect();
        assert!(hashes.contains(&"hash_2"));
        assert!(hashes.contains(&"hash_5"));
    }

    #[tokio::test]
    async fn an_unlinked_canonical_suffix_leaves_the_canonical_set() {
        let datastore = modality_datastore::DatastoreManager::create_in_memory().unwrap();
        for block in [
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ] {
            block.save_to_active(&datastore).await.unwrap();
        }
        let mut junk = make_test_block(4, "missing");
        junk.hash = "junk_4".to_string();
        junk.save_to_active(&datastore).await.unwrap();

        assert!(select_best_stored_chain(&datastore).await.unwrap());
        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<_> = canonical.iter().map(|block| block.hash.as_str()).collect();
        assert!(hashes.contains(&"hash_2"));
        assert!(!hashes.contains(&"junk_4"));
        let stored = MinerBlock::find_by_hash_multi(&datastore, "junk_4")
            .await
            .unwrap()
            .unwrap();
        assert!(!stored.is_canonical);
        assert!(!stored.is_orphaned);
    }

    #[tokio::test]
    async fn a_later_nomination_gap_does_not_block_the_linked_prefix() {
        let mut datastore = modality_datastore::DatastoreManager::create_in_memory().unwrap();
        datastore.set_blocks_per_epoch(2);
        let mut chain = [
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        chain[2].epoch = 2;
        for block in chain {
            block.save_to_active(&datastore).await.unwrap();
        }
        let mut same = make_test_block(3, "hash_2");
        same.epoch = 2;
        same.is_canonical = false;
        same.actualized_difficulty = "5000".to_string();
        same.target_difficulty = "1000".to_string();
        same.save_to_active(&datastore).await.unwrap();
        let mut later = make_test_block(4, "hash_3");
        later.epoch = 4;
        later.is_canonical = false;
        later.actualized_difficulty = "5000".to_string();
        later.target_difficulty = "1000".to_string();
        later.save_to_active(&datastore).await.unwrap();

        assert!(adopt_connected_extensions(&datastore).await.unwrap() >= 1);
        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<_> = canonical.iter().map(|block| block.hash.as_str()).collect();
        assert!(hashes.contains(&"hash_3"));
        assert!(!hashes.contains(&"hash_4"));
        let parked = MinerBlock::find_by_hash_multi(&datastore, "hash_4")
            .await
            .unwrap()
            .unwrap();
        assert!(!parked.is_canonical);
        assert!(!parked.is_orphaned);
    }
}
