//! Fork choice on the datastore's miner tree.
//!
//! `reorg::select_best_stored_chain` and `reorg::adopt_connected_extensions`
//! decoded and rescored every stored block for each gossiped block, under
//! the datastore lock. The tree keeps each live block's linked work and the
//! best tips between calls, so a decision here walks only the two suffixes
//! above their fork point, and the linked branch tips.
//!
//! The rules are the ones those functions apply, in the same order and with
//! the same log lines: the sequenced-prefix floor, the nomination epoch,
//! then verified work. A plan is computed under the tree lock and the
//! writes are applied afterwards. Where the tree cannot answer (no anchored
//! canonical spine, a proof that misses its target), the planner says so and
//! the caller runs the full scan.

use crate::chain::fork_choice::{
    check_expected_target_with, choose_verified, proof_meets_target, ChainWork, ForkChoiceResult,
    RetargetParams, TargetCheck,
};
use modality_datastore::models::miner::LinkedWork;
use anyhow::Result;
use modality_datastore::models::miner::{Eligibility, MinerTree};
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// A refused branch that forks this many epochs under the local tip is
/// archived instead of being scored again.
pub(crate) const STALE_FORK_EPOCHS: u64 = 2;

pub fn retarget_params(mgr: &DatastoreManager) -> RetargetParams {
    RetargetParams {
        blocks_per_epoch: mgr.epoch_config().blocks_per_epoch,
        target_block_time_secs: mgr.network_u64("target_block_time_secs").unwrap_or(60),
        initial_difficulty: mgr
            .network_u64("initial_difficulty")
            .map(|value| value as u128),
    }
}

/// Run `f` on the tree after syncing it with the store. The target check is
/// the eligibility rule, as `select_best_stored_chain` filtered on it.
pub fn with_synced_tree<R>(
    mgr: &DatastoreManager,
    params: RetargetParams,
    f: impl FnOnce(&MinerTree) -> R,
) -> Result<R> {
    let mut tree = mgr.miner_tree();
    mgr.with_miner_cache(|cache| {
        tree.sync(cache, &mut |block, live| {
            match check_expected_target_with(block, |hash| live.get(hash).map(|b| b.as_ref()), params)
            {
                TargetCheck::Matches => Eligibility::Eligible,
                TargetCheck::Reject => Eligibility::Ineligible,
                TargetCheck::Unresolved => Eligibility::EligibleForNow,
            }
        });
    })?;
    Ok(f(&tree))
}

/// Which stored blocks the target check may walk through.
#[derive(Clone, Copy)]
pub(crate) enum Ancestors {
    /// Every stored block, as the gossip handler checked.
    Stored,
    /// Blocks that are not orphaned, as parent sync checked.
    Live,
}

/// The target check for a block that is not stored yet, against the
/// stored blocks, without the full scan it used to take.
pub(crate) fn check_target_against_store(
    mgr: &DatastoreManager,
    block: &MinerBlock,
    params: RetargetParams,
    ancestors: Ancestors,
) -> Result<TargetCheck> {
    Ok(mgr.with_miner_cache(|cache| {
        check_expected_target_with(
            block,
            |hash| {
                cache
                    .effective(hash)
                    .filter(|b| matches!(ancestors, Ancestors::Stored) || !b.is_orphaned)
                    .map(|b| b.as_ref())
            },
            params,
        )
    })?)
}

/// Flag writes a plan makes, keyed by hash so a later write to the same
/// block replaces an earlier one.
#[derive(Default)]
pub(crate) struct Writes(BTreeMap<String, MinerBlock>);

impl Writes {
    fn current<'a>(&'a self, block: &'a MinerBlock) -> &'a MinerBlock {
        self.0.get(&block.hash).unwrap_or(block)
    }

    fn set(&mut self, block: MinerBlock) {
        self.0.insert(block.hash.clone(), block);
    }

    fn promote(&mut self, block: &MinerBlock) {
        let mut promoted = self.current(block).clone();
        promoted.is_canonical = true;
        promoted.is_orphaned = false;
        promoted.orphan_reason = None;
        self.set(promoted);
    }

    fn demote(&mut self, block: &MinerBlock, reason: &str) {
        let mut competing = self.current(block).clone();
        competing.is_canonical = false;
        competing.is_orphaned = false;
        competing.orphan_reason = Some(reason.to_string());
        self.set(competing);
    }

    fn orphan(&mut self, block: &MinerBlock, reason: String, competing_hash: Option<String>) {
        let mut orphaned = self.current(block).clone();
        orphaned.mark_as_orphaned(reason, competing_hash);
        self.set(orphaned);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Save every write, lowest index first.
    pub(crate) async fn apply(self, mgr: &DatastoreManager) -> Result<()> {
        let mut blocks: Vec<MinerBlock> = self.0.into_values().collect();
        blocks.sort_by_key(|block| block.index);
        for block in blocks {
            block.save_to_active(mgr).await?;
        }
        Ok(())
    }
}

/// The blocks of a linked walk from `tip` down to, not including, `stop`
/// (or to index 0), highest first.
fn walk_down<'a>(tree: &'a MinerTree, tip: &'a Arc<MinerBlock>, stop: Option<&str>) -> Vec<&'a Arc<MinerBlock>> {
    let mut out = Vec::new();
    let mut current = tip;
    loop {
        if stop == Some(current.hash.as_str()) {
            break;
        }
        out.push(current);
        match tree.parent(&current.hash) {
            Some(parent) => current = parent,
            None => break,
        }
    }
    out
}

/// The highest block both walks share, or `None` when they do not meet.
fn fork_point<'a>(
    tree: &'a MinerTree,
    left: &'a Arc<MinerBlock>,
    right: &'a Arc<MinerBlock>,
) -> Option<&'a Arc<MinerBlock>> {
    let (mut a, mut b) = (left, right);
    while a.index > b.index {
        a = tree.parent(&a.hash)?;
    }
    while b.index > a.index {
        b = tree.parent(&b.hash)?;
    }
    while a.hash != b.hash {
        a = tree.parent(&a.hash)?;
        b = tree.parent(&b.hash)?;
    }
    Some(a)
}

/// True when the walk to `tip` holds `block`.
fn on_walk(tree: &MinerTree, tip: &Arc<MinerBlock>, block: &MinerBlock) -> bool {
    let mut current = tip;
    while current.index > block.index {
        match tree.parent(&current.hash) {
            Some(parent) => current = parent,
            None => return false,
        }
    }
    current.hash == block.hash
}

/// A walk from index 0 to `tip` holds every block of the nomination epoch
/// for `tip`'s epoch. This is `nomination_epoch_complete` for a walk that
/// has one block at every index below its tip.
fn covers_nomination_epoch(tip: &MinerBlock, blocks_per_epoch: u64) -> bool {
    if tip.epoch < 2 || blocks_per_epoch == 0 {
        return true;
    }
    let start = (tip.epoch - 2).saturating_mul(blocks_per_epoch);
    tip.index.saturating_add(1) >= start.saturating_add(blocks_per_epoch)
}

/// Spine work the node's scoring compares: linked when every mined block on
/// the walk meets its target, unknown otherwise.
fn scored(work: LinkedWork) -> ChainWork {
    if work.proofs_ok {
        ChainWork::Linked(work.work)
    } else {
        ChainWork::Unknown
    }
}

pub(crate) enum Plan {
    /// The tree cannot decide this case; run the full scan.
    Scan,
    /// Apply these writes. `changed` is the value the scan would return.
    Apply { writes: Writes, changed: usize },
}

/// `select_best_stored_chain` on the tree.
pub(crate) fn plan_select(tree: &MinerTree, floor: Option<u64>, blocks_per_epoch: u64) -> Plan {
    let Some((winner, winner_work)) = tree.best() else {
        return Plan::Scan;
    };
    let Some((local, local_work)) = tree.canonical_best() else {
        return Plan::Scan;
    };
    // A checkpoint or nomination epoch on a chain whose work is unknown
    // protects nothing that can be verified.
    let local_anchored = local_work.proofs_ok;
    let mut writes = Writes::default();

    // Canonical blocks off the canonical spine leave the canonical set,
    // except under the sequenced prefix.
    let mut kept_off_spine: Vec<&Arc<MinerBlock>> = Vec::new();
    if tree.canonical_count() != (local.index + 1) as usize {
        let spine: HashSet<&str> = walk_down(tree, local, None)
            .into_iter()
            .map(|block| block.hash.as_str())
            .collect();
        for hash in tree.canonical_hashes() {
            if spine.contains(hash.as_str()) {
                continue;
            }
            let Some(block) = tree.node(hash) else {
                continue;
            };
            if floor.is_some_and(|floor| block.index <= floor) {
                kept_off_spine.push(block);
            } else {
                writes.demote(block, "Off the verified spine");
            }
        }
    }
    let demoted = usize::from(!writes.is_empty());
    let keep = |writes| Plan::Apply {
        writes,
        changed: demoted,
    };

    if winner.hash == local.hash && kept_off_spine.is_empty() {
        return keep(writes);
    }
    let Some(fork) = fork_point(tree, winner, local) else {
        return Plan::Scan;
    };

    if let Some(floor) = floor.filter(|_| local_anchored) {
        let spine_under_floor = local.index > fork.index && fork.index < floor;
        let kept_under_floor = kept_off_spine
            .iter()
            .any(|block| block.index <= floor && !on_walk(tree, winner, block));
        if spine_under_floor || kept_under_floor {
            log::warn!("Refusing to switch off the sequenced prefix through {floor}");
            return keep(writes);
        }
    }
    if local_anchored
        && covers_nomination_epoch(local, blocks_per_epoch)
        && !covers_nomination_epoch(winner, blocks_per_epoch)
    {
        log::warn!("Refusing a fork that drops the nomination epoch");
        return keep(writes);
    }

    let decision = choose_verified(
        scored(local_work),
        local.index,
        &local.hash,
        scored(winner_work),
        winner.index,
        &winner.hash,
    );
    let fills_parents = fork.hash == local.hash
        && kept_off_spine
            .iter()
            .all(|block| on_walk(tree, winner, block));
    if decision.result != ForkChoiceResult::AdoptRemote && !fills_parents {
        log::info!(
            "Keeping local chain at {} ({:?}) over stored spine at {} ({:?}): {}",
            local.index,
            scored(local_work),
            winner.index,
            scored(winner_work),
            decision.reason
        );
        return keep(writes);
    }
    log::info!(
        "Switching to stored spine at {} ({:?}) from local {} ({:?})",
        winner.index,
        scored(winner_work),
        local.index,
        scored(local_work)
    );
    for block in walk_down(tree, winner, Some(&fork.hash)) {
        if !writes.current(block).is_canonical {
            writes.promote(block);
        }
    }
    for block in walk_down(tree, local, Some(&fork.hash)) {
        writes.demote(block, "Competing fork");
    }
    for block in kept_off_spine {
        if !on_walk(tree, winner, block) {
            writes.demote(block, "Competing fork");
        }
    }
    Plan::Apply { writes, changed: 1 }
}

/// One pass of `adopt_connected_extensions` on the tree: every linked branch
/// tip off the canonical spine is decided against the canonical chain. The
/// pass stops at the first adoption, since that changes the canonical
/// chain; the caller plans again.
pub(crate) fn plan_adopt(tree: &MinerTree, floor: Option<u64>, blocks_per_epoch: u64) -> Plan {
    let Some((local, local_work)) = tree.canonical_best() else {
        return Plan::Scan;
    };
    if tree.canonical_count() != (local.index + 1) as usize {
        return Plan::Scan;
    }
    let local_anchored = local_work.proofs_ok;
    let local_covers = covers_nomination_epoch(local, blocks_per_epoch);
    let mut leaves = tree.linked_leaves();
    leaves.sort_by(|a, b| {
        let work = |block: &MinerBlock| tree.linked(&block.hash).map(|w| w.work).unwrap_or(0);
        work(b)
            .cmp(&work(a))
            .then(b.index.cmp(&a.index))
            .then(a.hash.cmp(&b.hash))
    });

    let mut writes = Writes::default();
    for leaf in leaves {
        if leaf.is_canonical {
            continue;
        }
        // The branch above its highest canonical block, lowest first.
        let mut extension: Vec<&Arc<MinerBlock>> = Vec::new();
        let mut current = leaf;
        let ancestor = loop {
            if current.is_canonical {
                break Some(current);
            }
            extension.push(current);
            match tree.parent(&current.hash) {
                Some(parent) => current = parent,
                None => break None,
            }
        };
        let Some(ancestor) = ancestor else {
            continue;
        };
        extension.reverse();
        if extension.is_empty() {
            continue;
        }

        let safe_len = if local_covers {
            extension
                .iter()
                .take_while(|block| covers_nomination_epoch(block, blocks_per_epoch))
                .count()
        } else {
            extension.len()
        };
        let bottom = extension[0].index;
        if safe_len == 0 {
            let tip = extension[extension.len() - 1].index;
            log::info!("Keeping parked extension {bottom}..={tip}: nomination epoch");
            continue;
        }
        let safe = &extension[..safe_len];
        let safe_tip = safe[safe_len - 1];

        let floor_refusal = floor.filter(|floor| {
            local_anchored && local.index > ancestor.index && ancestor.index < *floor
        });
        let refusal = if let Some(floor) = floor_refusal {
            Err(format!("Refusing to orphan the sequenced prefix through {floor}"))
        } else {
            // Local work above the ancestor counts up to the first block that
            // misses its target, as `work_above_ancestor` walks it.
            let mut local_top: Option<&Arc<MinerBlock>> = None;
            for block in walk_down(tree, local, Some(&ancestor.hash)).into_iter().rev() {
                if !proof_meets_target(block) {
                    break;
                }
                local_top = Some(block);
            }
            let ancestor_work = tree.canonical_linked(&ancestor.hash).map(|w| w.work);
            let (local_chain_work, local_tip, local_hash) =
                match (local_top, ancestor_work) {
                    (Some(top), Some(below)) => (
                        ChainWork::Linked(
                            tree.canonical_linked(&top.hash)
                                .map(|w| w.work)
                                .unwrap_or(below)
                                .saturating_sub(below),
                        ),
                        top.index,
                        top.hash.clone(),
                    ),
                    _ => (ChainWork::Unknown, local.index, local.hash.clone()),
                };
            let suffix_meets = safe.iter().all(|block| proof_meets_target(block));
            let remote_work = match (tree.linked(&safe_tip.hash), tree.linked(&ancestor.hash)) {
                (Some(tip), Some(below)) if suffix_meets => {
                    ChainWork::Linked(tip.work.saturating_sub(below.work))
                }
                _ => ChainWork::Unknown,
            };
            let comparison = choose_verified(
                local_chain_work,
                local_tip,
                &local_hash,
                remote_work,
                safe_tip.index,
                &safe_tip.hash,
            );
            if comparison.result == ForkChoiceResult::AdoptRemote {
                Ok(comparison.reason)
            } else {
                Err(comparison.reason)
            }
        };

        match refusal {
            Err(reason) => {
                log::info!(
                    "Keeping parked extension {bottom}..={}: {reason}",
                    safe_tip.index
                );
                if bottom.saturating_add(STALE_FORK_EPOCHS.saturating_mul(blocks_per_epoch))
                    <= local.index
                {
                    for block in &extension {
                        writes.orphan(
                            block,
                            format!("Stale fork {} epochs under the tip", STALE_FORK_EPOCHS),
                            None,
                        );
                    }
                    log::info!(
                        "Archived stale fork {bottom}..={} under local tip {}",
                        leaf.index,
                        local.index
                    );
                }
            }
            Ok(reason) => {
                log::info!("Adopting linked extension: {reason}");
                for block in walk_down(tree, local, Some(&ancestor.hash)) {
                    let competing = safe
                        .iter()
                        .find(|candidate| candidate.index == block.index)
                        .map(|candidate| candidate.hash.clone());
                    writes.orphan(block, reason.clone(), competing);
                }
                for block in safe {
                    writes.promote(block);
                }
                return Plan::Apply {
                    writes,
                    changed: safe_len,
                };
            }
        }
    }
    Plan::Apply { writes, changed: 0 }
}

#[cfg(test)]
mod tests {
    use crate::chain::reorg::{
        adopt_connected_extensions, select_best_stored_chain, select_best_stored_chain_scan,
    };
    use modality_datastore::models::MinerBlock;
    use modality_datastore::DatastoreManager;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    struct Shape {
        blocks_per_epoch: u64,
        strays: bool,
        bad_proofs: bool,
    }

    fn block(hash: String, index: u64, previous: &str, actual: u64, blocks_per_epoch: u64) -> MinerBlock {
        let mut block = MinerBlock::new_canonical(
            hash,
            index,
            index / blocks_per_epoch,
            1_700_000_000 + index as i64 * 60,
            previous.to_string(),
            "data".to_string(),
            1,
            1000,
            "peer".to_string(),
            1,
        );
        block.actualized_difficulty = actual.to_string();
        block.is_canonical = false;
        block
    }

    /// A random store: a canonical chain, forks off it, parked runs whose
    /// parent is missing, and orphans; with `strays`, canonical flags off
    /// that chain; with `bad_proofs`, a few blocks that miss their target.
    fn random_store(rng: &mut StdRng, shape: &Shape) -> Vec<MinerBlock> {
        let bpe = shape.blocks_per_epoch;
        let mut blocks = vec![block("g".into(), 0, "", 1000, bpe)];
        blocks[0].is_canonical = true;
        let n = rng.gen_range(5..60);
        for i in 0..n {
            let parent = blocks[rng.gen_range(0..blocks.len())].clone();
            let parked = rng.gen_bool(0.08);
            let previous = if parked {
                format!("missing{i}")
            } else {
                parent.hash.clone()
            };
            let actual = if shape.bad_proofs && rng.gen_bool(0.04) {
                rng.gen_range(1..999)
            } else {
                rng.gen_range(1000..6000)
            };
            let mut child = block(format!("b{i:03}"), parent.index + 1, &previous, actual, bpe);
            child.is_orphaned = rng.gen_bool(0.08);
            blocks.push(child);
        }
        // Flag one walk canonical, from a random block down to genesis.
        let mut current = blocks[rng.gen_range(0..blocks.len())].hash.clone();
        let mut walk = Vec::new();
        while let Some(at) = blocks.iter().position(|b| b.hash == current) {
            if blocks[at].is_orphaned {
                walk.clear();
                break;
            }
            walk.push(at);
            if blocks[at].index == 0 {
                break;
            }
            current = blocks[at].previous_hash.clone();
        }
        for at in walk {
            blocks[at].is_canonical = true;
        }
        if shape.strays && rng.gen_bool(0.3) {
            let at = rng.gen_range(0..blocks.len());
            if !blocks[at].is_orphaned {
                blocks[at].is_canonical = true;
            }
        }
        blocks
    }

    async fn store_of(blocks: &[MinerBlock], blocks_per_epoch: u64) -> DatastoreManager {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.set_blocks_per_epoch(blocks_per_epoch);
        for block in blocks {
            block.save_to_active(&mgr).await.unwrap();
        }
        mgr
    }

    async fn state(mgr: &DatastoreManager) -> Vec<(String, bool, bool)> {
        let mut out: Vec<_> = MinerBlock::find_all_blocks_multi(mgr)
            .await
            .unwrap()
            .into_iter()
            .map(|b| (b.hash, b.is_canonical, b.is_orphaned))
            .collect();
        out.sort();
        out
    }

    fn diff(case: usize, blocks: &[MinerBlock], fast: &[(String, bool, bool)], scan: &[(String, bool, bool)]) -> String {
        let mut out = format!("case {case}\n");
        for b in blocks {
            out.push_str(&format!(
                "  {} i{} prev={} act={} canon={} orph={}\n",
                b.hash, b.index, b.previous_hash, b.actualized_difficulty, b.is_canonical, b.is_orphaned
            ));
        }
        for (f, s) in fast.iter().zip(scan) {
            if f != s {
                out.push_str(&format!("  fast {f:?} scan {s:?}\n"));
            }
        }
        out
    }

    /// One select pass gives what the full scan gives.
    ///
    /// Stray canonical flags are left out: there the full scan decides from
    /// the store as it read it at the start of the pass, so a block it
    /// demotes off the canonical spine that is also on the winner stays
    /// demoted, leaving a hole under the canonical tip.
    /// `tree_choice_follows_the_verified_spine` covers those stores.
    #[tokio::test]
    async fn tree_select_matches_the_full_scan() {
        let shape = Shape {
            blocks_per_epoch: 4,
            strays: false,
            bad_proofs: true,
        };
        let mut rng = StdRng::seed_from_u64(7);
        for case in 0..600 {
            let blocks = random_store(&mut rng, &shape);
            let fast = store_of(&blocks, shape.blocks_per_epoch).await;
            let scan = store_of(&blocks, shape.blocks_per_epoch).await;
            // A sequenced prefix through a random canonical index, now and then.
            if rng.gen_bool(0.4) {
                let canonical: Vec<&MinerBlock> = blocks.iter().filter(|b| b.is_canonical).collect();
                let at = canonical[rng.gen_range(0..canonical.len())];
                let checkpoint = modality_datastore::models::miner::MinerCheckpoint::new_manual(
                    0,
                    at.index,
                    at.hash.clone(),
                    String::new(),
                    at.index + 1,
                    None,
                );
                checkpoint.save_to_canon(&fast).await.unwrap();
                checkpoint.save_to_canon(&scan).await.unwrap();
            }
            let fast_switched = select_best_stored_chain(&fast).await.unwrap();
            let scan_switched = select_best_stored_chain_scan(&scan).await.unwrap();
            let (f, s) = (state(&fast).await, state(&scan).await);
            assert!(f == s, "{}", diff(case, &blocks, &f, &s));
            assert_eq!(fast_switched, scan_switched, "case {case}: select result");
        }
    }

    /// After a select pass and an adoption pass, with no nomination epoch or
    /// sequenced prefix in play: the canonical set is one walk from genesis,
    /// no live branch beats it by the adoption rule (work above the fork
    /// point, every block of the branch meeting its target), and another
    /// pass changes nothing.
    ///
    /// The full scan's adoption pass is not the reference here. It takes
    /// extensions in store order, so it can adopt a lighter branch first and
    /// then refuse the heavier one, which no longer links.
    #[tokio::test]
    async fn tree_adoption_leaves_no_branch_that_beats_the_chain() {
        use crate::chain::fork_choice::{choose_verified, proof_meets_target, ChainWork, ForkChoiceResult};
        let shape = Shape {
            blocks_per_epoch: 1000,
            strays: false,
            bad_proofs: true,
        };
        let mut rng = StdRng::seed_from_u64(13);
        for case in 0..600 {
            let blocks = random_store(&mut rng, &shape);
            let fast = store_of(&blocks, shape.blocks_per_epoch).await;
            select_best_stored_chain(&fast).await.unwrap();
            adopt_connected_extensions(&fast).await.unwrap();

            let all = MinerBlock::find_all_blocks_multi(&fast).await.unwrap();
            let live: Vec<&MinerBlock> = all.iter().filter(|b| !b.is_orphaned).collect();
            let by_hash: std::collections::HashMap<&str, &MinerBlock> =
                live.iter().map(|b| (b.hash.as_str(), *b)).collect();
            let parent = |b: &MinerBlock| {
                by_hash
                    .get(b.previous_hash.as_str())
                    .copied()
                    .filter(|p| b.index > 0 && p.index + 1 == b.index)
            };
            let walk = |tip: &MinerBlock| -> Option<Vec<&MinerBlock>> {
                let mut path = vec![by_hash[tip.hash.as_str()]];
                while path.last().unwrap().index > 0 {
                    path.push(parent(path.last().unwrap())?);
                }
                path.reverse();
                Some(path)
            };
            let canonical: Vec<&MinerBlock> = live.iter().copied().filter(|b| b.is_canonical).collect();
            let tip = *canonical.iter().max_by_key(|b| b.index).unwrap();
            let spine = walk(tip).expect("canonical tip links to genesis");
            assert!(
                spine.iter().all(|b| b.is_canonical) && spine.len() == canonical.len(),
                "case {case}: canonical set is not one walk\n{}",
                diff(case, &blocks, &[], &[])
            );

            let work = |path: &[&MinerBlock]| -> ChainWork {
                if path.iter().all(|b| proof_meets_target(b)) {
                    ChainWork::Linked(path.iter().map(|b| b.get_actualized_difficulty_u128().unwrap()).sum())
                } else {
                    ChainWork::Unknown
                }
            };
            for leaf in &live {
                let Some(path) = walk(leaf) else { continue };
                let is_leaf = !live.iter().any(|c| parent(c).is_some_and(|p| p.hash == leaf.hash));
                if !is_leaf || leaf.is_canonical {
                    continue;
                }
                let at = path.iter().rposition(|b| b.is_canonical).unwrap();
                let suffix = &path[at + 1..];
                let above = &spine[at + 1..];
                // Local work counts up to the first block that misses its target.
                let ok = above.iter().take_while(|b| proof_meets_target(b)).count();
                let (local_work, local_tip, local_hash) = if ok > 0 {
                    let top = above[ok - 1];
                    (work(&above[..ok]), top.index, top.hash.as_str())
                } else {
                    (ChainWork::Unknown, tip.index, tip.hash.as_str())
                };
                let remote_tip = suffix.last().unwrap();
                let decision = choose_verified(
                    local_work,
                    local_tip,
                    local_hash,
                    work(suffix),
                    remote_tip.index,
                    &remote_tip.hash,
                );
                assert!(
                    decision.result != ForkChoiceResult::AdoptRemote,
                    "case {case}: branch to {} beats the chain: {}\n{}",
                    leaf.hash,
                    decision.reason,
                    diff(case, &blocks, &[], &[])
                );
            }
            assert_eq!(
                adopt_connected_extensions(&fast).await.unwrap(),
                0,
                "case {case}: a second adoption pass adopted"
            );
        }
    }

    /// With no nomination epoch in play, one pass leaves exactly the
    /// verified spine of the live blocks canonical, and a second pass
    /// changes nothing. Stray canonical flags included.
    #[tokio::test]
    async fn tree_choice_follows_the_verified_spine() {
        let shape = Shape {
            blocks_per_epoch: 1000,
            strays: true,
            bad_proofs: false,
        };
        let mut rng = StdRng::seed_from_u64(11);
        for case in 0..600 {
            let blocks = random_store(&mut rng, &shape);
            let live: Vec<MinerBlock> = blocks.iter().filter(|b| !b.is_orphaned).cloned().collect();
            let mut expected: Vec<String> =
                MinerBlock::verified_spine(&live).into_iter().map(|b| b.hash).collect();
            expected.sort();
            let fast = store_of(&blocks, shape.blocks_per_epoch).await;
            select_best_stored_chain(&fast).await.unwrap();
            adopt_connected_extensions(&fast).await.unwrap();
            let mut canonical: Vec<String> = MinerBlock::find_all_canonical_multi(&fast)
                .await
                .unwrap()
                .into_iter()
                .map(|b| b.hash)
                .collect();
            canonical.sort();
            assert_eq!(canonical, expected, "case {case}: {blocks:#?}");
            assert!(
                !select_best_stored_chain(&fast).await.unwrap(),
                "case {case}: a second pass changed the chain"
            );
        }
    }

    /// The incremental tree, synced across many writes, agrees with a tree
    /// rebuilt from scratch.
    #[tokio::test]
    async fn an_incremental_tree_matches_a_rebuilt_one() {
        let mut rng = StdRng::seed_from_u64(3);
        for case in 0..100 {
            let shape = Shape {
                blocks_per_epoch: 4,
                strays: true,
                bad_proofs: true,
            };
            let blocks = random_store(&mut rng, &shape);
            let mgr = store_of(&blocks[..1], shape.blocks_per_epoch).await;
            let params = super::retarget_params(&mgr);
            for block in &blocks[1..] {
                block.save_to_active(&mgr).await.unwrap();
                super::with_synced_tree(&mgr, params, |_| ()).unwrap();
            }
            select_best_stored_chain(&mgr).await.unwrap();
            adopt_connected_extensions(&mgr).await.unwrap();
            let incremental = super::with_synced_tree(&mgr, params, |tree| {
                {
                    let mut leaves: Vec<String> =
                        tree.linked_leaves().iter().map(|b| b.hash.clone()).collect();
                    leaves.sort();
                    (
                        tree.best().map(|(b, w)| (b.hash.clone(), w)),
                        tree.canonical_best().map(|(b, w)| (b.hash.clone(), w)),
                        tree.canonical_count(),
                        leaves,
                    )
                }
            })
            .unwrap();
            mgr.invalidate_miner_cache();
            let rebuilt = super::with_synced_tree(&mgr, params, |tree| {
                {
                    let mut leaves: Vec<String> =
                        tree.linked_leaves().iter().map(|b| b.hash.clone()).collect();
                    leaves.sort();
                    (
                        tree.best().map(|(b, w)| (b.hash.clone(), w)),
                        tree.canonical_best().map(|(b, w)| (b.hash.clone(), w)),
                        tree.canonical_count(),
                        leaves,
                    )
                }
            })
            .unwrap();
            assert_eq!(incremental, rebuilt, "case {case}");
        }
    }

    /// A store of `len` blocks on one canonical chain, with a short dead
    /// fork every 50 blocks and a parked block every 100.
    async fn long_store(len: u64) -> (DatastoreManager, MinerBlock) {
        let bpe = 40;
        let mgr = store_of(&[], bpe).await;
        let mut previous = block("g".into(), 0, "", 1000, bpe);
        previous.is_canonical = true;
        previous.save_to_active(&mgr).await.unwrap();
        for index in 1..len {
            let mut next = block(format!("c{index:06}"), index, &previous.hash, 2000, bpe);
            next.is_canonical = true;
            next.save_to_active(&mgr).await.unwrap();
            if index % 50 == 0 {
                block(format!("f{index:06}"), index + 1, &next.hash, 1500, bpe)
                    .save_to_active(&mgr)
                    .await
                    .unwrap();
            }
            if index % 100 == 0 {
                block(format!("p{index:06}"), index + 5, "nowhere", 3000, bpe)
                    .save_to_active(&mgr)
                    .await
                    .unwrap();
            }
            previous = next;
        }
        (mgr, previous)
    }

    /// What the gossip handler does for one block, after its checks.
    async fn score_one(mgr: &DatastoreManager, previous: &MinerBlock, n: u64) -> MinerBlock {
        let next = block(format!("n{n:06}"), previous.index + 1, &previous.hash, 2000, 40);
        next.save_to_active(mgr).await.unwrap();
        let params = super::retarget_params(mgr);
        super::with_synced_tree(mgr, params, |tree| tree.linked(&next.hash).is_some()).unwrap();
        select_best_stored_chain(mgr).await.unwrap();
        adopt_connected_extensions(mgr).await.unwrap();
        let tip = crate::chain::reorg::canonical_tip_index(mgr).await.unwrap();
        assert_eq!(tip, Some(next.index));
        next
    }

    async fn time_per_block(len: u64) -> std::time::Duration {
        let (mgr, mut previous) = long_store(len).await;
        // The first pass builds the tree from the store once.
        previous = score_one(&mgr, &previous, 0).await;
        let started = std::time::Instant::now();
        let rounds = 40;
        for n in 1..=rounds {
            previous = score_one(&mgr, &previous, n).await;
        }
        started.elapsed() / rounds as u32
    }

    /// Scoring a gossiped block costs about the same on a store ten times
    /// larger. The full scan cost grew faster than the store; the testnet
    /// spent minutes per block at about 20k blocks.
    #[tokio::test]
    async fn scoring_a_gossiped_block_does_not_grow_with_the_store() {
        let small = time_per_block(2_000).await;
        let large = time_per_block(20_000).await;
        eprintln!("per gossiped block: {small:?} at 2k blocks, {large:?} at 20k blocks");
        assert!(
            large < small * 3 + std::time::Duration::from_millis(5),
            "{large:?} at 20k blocks against {small:?} at 2k"
        );
    }
}
