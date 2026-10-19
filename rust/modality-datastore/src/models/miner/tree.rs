//! Fork-choice tree over the live miner blocks.
//!
//! `MinerBlock::verified_spine` rebuilt this from every stored block on each
//! call. The tree keeps it between calls. A sync reads the hashes the cache
//! journal says changed, and relinks only those blocks and the descendants
//! whose result moved.
//!
//! Two spines are tracked, by the same rule `verified_spine` applies: a
//! parent-linked walk from an index-0 block, most work, then the higher tip,
//! then the lower tip hash.
//!
//! - The winner spine runs over live blocks (not orphaned) that pass the
//!   caller's eligibility check (the target check, in the node).
//! - The canonical spine runs over live blocks flagged canonical, with no
//!   eligibility check, as `verified_spine(canonical)` did.
//!
//! Each linked block also records whether every mined block under it meets
//! its claimed target, which is what turns spine work into `Linked` work in
//! the node's scoring.

use crate::models::miner::{MinerBlock, MinerBlockCache};
use std::cmp::Reverse;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// Work of the parent-linked walk from index 0 to a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedWork {
    /// Sum of actualized difficulty, genesis included, as `verified_spine`
    /// ranks spines.
    pub work: u128,
    /// Every block above index 0 on the walk meets its claimed target.
    pub proofs_ok: bool,
}

/// Whether a block may be on the winner spine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eligibility {
    /// Eligible, and the answer cannot change: it is kept for the hash.
    Eligible,
    /// Eligible for now; asked again the next time the block is relinked.
    EligibleForNow,
    /// Never eligible. Kept for the hash.
    Ineligible,
}

type Rank = (u128, u64, Reverse<String>);

fn rank(block: &MinerBlock, linked: LinkedWork) -> Rank {
    (linked.work, block.index, Reverse(block.hash.clone()))
}

fn meets_target(block: &MinerBlock) -> bool {
    let Ok(actual) = block.get_actualized_difficulty_u128() else {
        return false;
    };
    let Ok(target) = block.target_difficulty.parse::<u128>() else {
        return false;
    };
    target > 0 && actual >= target
}

#[derive(Default)]
pub struct MinerTree {
    synced: Option<(u64, u64)>,
    nodes: HashMap<String, Arc<MinerBlock>>,
    children: HashMap<String, Vec<String>>,
    linked: HashMap<String, (LinkedWork, Rank)>,
    ranked: BTreeSet<Rank>,
    canonical: HashSet<String>,
    clinked: HashMap<String, (LinkedWork, Rank)>,
    cranked: BTreeSet<Rank>,
    verdicts: HashMap<String, bool>,
    /// Parent of each linked block, and how many linked children each
    /// linked block has; a linked block with none is a leaf.
    linked_parent: HashMap<String, String>,
    linked_children: HashMap<String, usize>,
    leaves: HashSet<String>,
}

impl MinerTree {
    /// Bring the tree up to the cache. `eligibility` decides whether a block
    /// whose parent is on a linked winner walk may extend it; it gets the
    /// live blocks by hash to walk ancestors.
    pub fn sync<E>(&mut self, cache: &MinerBlockCache, eligibility: &mut E)
    where
        E: FnMut(&MinerBlock, &HashMap<String, Arc<MinerBlock>>) -> Eligibility,
    {
        let changes = self
            .synced
            .and_then(|(generation, version)| cache.changes_since(generation, version));
        match changes {
            Some(changed) => {
                let mut dirty = Vec::with_capacity(changed.len());
                for hash in changed {
                    self.apply(cache, &hash, &mut dirty);
                }
                self.relink(dirty, eligibility);
            }
            None => self.rebuild(cache, eligibility),
        }
        self.synced = Some((cache.generation(), cache.version()));
    }

    fn rebuild<E>(&mut self, cache: &MinerBlockCache, eligibility: &mut E)
    where
        E: FnMut(&MinerBlock, &HashMap<String, Arc<MinerBlock>>) -> Eligibility,
    {
        self.nodes.clear();
        self.children.clear();
        self.linked.clear();
        self.ranked.clear();
        self.canonical.clear();
        self.clinked.clear();
        self.cranked.clear();
        self.linked_parent.clear();
        self.linked_children.clear();
        self.leaves.clear();
        let mut dirty = Vec::new();
        for block in cache.effective_all() {
            if block.is_orphaned {
                continue;
            }
            self.insert_node(block.clone());
            dirty.push(block.hash.clone());
        }
        self.verdicts.retain(|hash, _| self.nodes.contains_key(hash));
        self.relink(dirty, eligibility);
    }

    fn insert_node(&mut self, block: Arc<MinerBlock>) {
        let siblings = self.children.entry(block.previous_hash.clone()).or_default();
        if !siblings.contains(&block.hash) {
            siblings.push(block.hash.clone());
        }
        if block.is_canonical {
            self.canonical.insert(block.hash.clone());
        }
        self.nodes.insert(block.hash.clone(), block);
    }

    fn apply(&mut self, cache: &MinerBlockCache, hash: &str, dirty: &mut Vec<String>) {
        if let Some(old) = self.nodes.remove(hash) {
            if let Some(siblings) = self.children.get_mut(&old.previous_hash) {
                siblings.retain(|child| child != hash);
                if siblings.is_empty() {
                    self.children.remove(&old.previous_hash);
                }
            }
            self.canonical.remove(hash);
        }
        if let Some(block) = cache.effective(hash).filter(|block| !block.is_orphaned) {
            self.insert_node(block.clone());
        }
        dirty.push(hash.to_string());
    }

    /// Recompute `start` and every descendant whose linked work changed,
    /// parents before children.
    fn relink<E>(&mut self, start: Vec<String>, eligibility: &mut E)
    where
        E: FnMut(&MinerBlock, &HashMap<String, Arc<MinerBlock>>) -> Eligibility,
    {
        let mut work: BTreeSet<(u64, String)> = BTreeSet::new();
        for hash in start {
            let index = self
                .nodes
                .get(&hash)
                .map(|block| block.index)
                .unwrap_or(0);
            work.insert((index, hash));
        }
        while let Some((_, hash)) = work.pop_first() {
            let linked = self.compute_linked(&hash, eligibility);
            let clinked = self.compute_clinked(&hash);
            let changed_winner =
                Self::set_linked(&mut self.linked, &mut self.ranked, &self.nodes, &hash, linked);
            if changed_winner {
                self.update_leaves(&hash, linked.is_some());
            }
            let changed_canonical =
                Self::set_linked(&mut self.clinked, &mut self.cranked, &self.nodes, &hash, clinked);
            let changed = changed_winner || changed_canonical;
            if changed {
                for child in self.children.get(&hash).into_iter().flatten() {
                    if let Some(block) = self.nodes.get(child) {
                        work.insert((block.index, child.clone()));
                    }
                }
            }
        }
    }

    fn set_linked(
        linked: &mut HashMap<String, (LinkedWork, Rank)>,
        ranked: &mut BTreeSet<Rank>,
        nodes: &HashMap<String, Arc<MinerBlock>>,
        hash: &str,
        new: Option<LinkedWork>,
    ) -> bool {
        if linked.get(hash).map(|(work, _)| *work) == new {
            return false;
        }
        if let Some((_, old_rank)) = linked.remove(hash) {
            ranked.remove(&old_rank);
        }
        if let (Some(new), Some(block)) = (new, nodes.get(hash)) {
            let new_rank = rank(block, new);
            ranked.insert(new_rank.clone());
            linked.insert(hash.to_string(), (new, new_rank));
        }
        true
    }

    /// Keep `leaves` after `hash` joined or left the winner walks.
    fn update_leaves(&mut self, hash: &str, now_linked: bool) {
        let was_linked = self.linked_parent.contains_key(hash) || self.leaves.contains(hash)
            || self.linked_children.contains_key(hash);
        if now_linked {
            let parent = self
                .nodes
                .get(hash)
                .filter(|block| block.index > 0)
                .map(|block| block.previous_hash.clone());
            if !was_linked {
                if let Some(parent) = &parent {
                    *self.linked_children.entry(parent.clone()).or_default() += 1;
                    self.leaves.remove(parent);
                    self.linked_parent.insert(hash.to_string(), parent.clone());
                }
                let children = self.linked_children.entry(hash.to_string()).or_default();
                if *children == 0 {
                    self.leaves.insert(hash.to_string());
                }
            }
            return;
        }
        if !was_linked {
            return;
        }
        self.leaves.remove(hash);
        self.linked_children.remove(hash);
        if let Some(parent) = self.linked_parent.remove(hash) {
            if let Some(count) = self.linked_children.get_mut(&parent) {
                *count = count.saturating_sub(1);
                if *count == 0 && self.linked.contains_key(&parent) {
                    self.leaves.insert(parent);
                }
            }
        }
    }

    fn parent_of(&self, block: &MinerBlock) -> Option<&Arc<MinerBlock>> {
        if block.index == 0 {
            return None;
        }
        self.nodes
            .get(&block.previous_hash)
            .filter(|parent| parent.index + 1 == block.index)
    }

    fn compute_linked<E>(&mut self, hash: &str, eligibility: &mut E) -> Option<LinkedWork>
    where
        E: FnMut(&MinerBlock, &HashMap<String, Arc<MinerBlock>>) -> Eligibility,
    {
        let block = self.nodes.get(hash)?.clone();
        let difficulty = block.get_actualized_difficulty_u128().ok()?;
        let below = if block.index == 0 {
            None
        } else {
            let parent = self.parent_of(&block)?;
            Some(self.linked.get(&parent.hash)?.0)
        };
        let eligible = match self.verdicts.get(hash) {
            Some(verdict) => *verdict,
            None => match eligibility(&block, &self.nodes) {
                Eligibility::Eligible => {
                    self.verdicts.insert(hash.to_string(), true);
                    true
                }
                Eligibility::Ineligible => {
                    self.verdicts.insert(hash.to_string(), false);
                    false
                }
                Eligibility::EligibleForNow => true,
            },
        };
        if !eligible {
            return None;
        }
        Some(match below {
            None => LinkedWork {
                work: difficulty,
                proofs_ok: true,
            },
            Some(parent) => LinkedWork {
                work: parent.work.checked_add(difficulty)?,
                proofs_ok: parent.proofs_ok && meets_target(&block),
            },
        })
    }

    fn compute_clinked(&self, hash: &str) -> Option<LinkedWork> {
        let block = self.nodes.get(hash)?;
        if !block.is_canonical {
            return None;
        }
        let difficulty = block.get_actualized_difficulty_u128().ok()?;
        if block.index == 0 {
            return Some(LinkedWork {
                work: difficulty,
                proofs_ok: true,
            });
        }
        let parent = self.parent_of(block)?;
        let below = self.clinked.get(&parent.hash)?.0;
        Some(LinkedWork {
            work: below.work.checked_add(difficulty)?,
            proofs_ok: below.proofs_ok && meets_target(block),
        })
    }

    /// Tip of the winner spine.
    pub fn best(&self) -> Option<(&Arc<MinerBlock>, LinkedWork)> {
        let (_, _, Reverse(hash)) = self.ranked.last()?;
        Some((self.nodes.get(hash)?, self.linked.get(hash)?.0))
    }

    /// Tip of the canonical spine, when it reaches index 0.
    pub fn canonical_best(&self) -> Option<(&Arc<MinerBlock>, LinkedWork)> {
        let (_, _, Reverse(hash)) = self.cranked.last()?;
        Some((self.nodes.get(hash)?, self.clinked.get(hash)?.0))
    }

    pub fn node(&self, hash: &str) -> Option<&Arc<MinerBlock>> {
        self.nodes.get(hash)
    }

    /// The live block a block's `previous_hash` names, one index lower.
    pub fn parent(&self, hash: &str) -> Option<&Arc<MinerBlock>> {
        self.parent_of(self.nodes.get(hash)?)
    }

    /// Winner-walk work of a block, when it is on a linked walk.
    pub fn linked(&self, hash: &str) -> Option<LinkedWork> {
        self.linked.get(hash).map(|(work, _)| *work)
    }

    /// Canonical-walk work of a block, when it is on one.
    pub fn canonical_linked(&self, hash: &str) -> Option<LinkedWork> {
        self.clinked.get(hash).map(|(work, _)| *work)
    }

    /// Live blocks flagged canonical.
    pub fn canonical_hashes(&self) -> impl Iterator<Item = &String> + '_ {
        self.canonical.iter()
    }

    pub fn canonical_count(&self) -> usize {
        self.canonical.len()
    }

    /// Linked blocks with no linked child: the tips of every winner-eligible
    /// branch.
    pub fn linked_leaves(&self) -> Vec<&Arc<MinerBlock>> {
        self.leaves
            .iter()
            .filter_map(|hash| self.nodes.get(hash))
            .collect()
    }

    pub fn live_count(&self) -> usize {
        self.nodes.len()
    }
}
