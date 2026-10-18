//! In-memory mirror of the three miner block stores.
//!
//! Fork choice used to decode every stored block from RocksDB several times
//! for each gossiped block. The cache holds the same records, keyed and
//! ordered the way each store iterates them, and every miner block write in
//! `multi_store` updates it after the store write succeeds. Readers that
//! scanned a store scan the matching map instead, so results do not change.
//!
//! Each change is also appended to a journal. A reader that keeps derived
//! state (the fork-choice tree) asks for the hashes changed since the
//! version it last saw, instead of rescanning.

use crate::models::miner::MinerBlock;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

/// Which store a cached record mirrors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinerStoreKind {
    Active,
    Forks,
    Canon,
}

/// Journal entries kept for readers that sync incrementally. A reader that
/// falls further behind than this rebuilds from the maps.
const JOURNAL_CAPACITY: usize = 200_000;

pub struct MinerBlockCache {
    active: BTreeMap<String, Arc<MinerBlock>>,
    forks: BTreeMap<String, Arc<MinerBlock>>,
    canon: BTreeMap<String, Arc<MinerBlock>>,
    generation: u64,
    version: u64,
    journal: VecDeque<(u64, String)>,
}

impl MinerBlockCache {
    pub(crate) fn new(
        generation: u64,
        active: BTreeMap<String, Arc<MinerBlock>>,
        forks: BTreeMap<String, Arc<MinerBlock>>,
        canon: BTreeMap<String, Arc<MinerBlock>>,
    ) -> Self {
        Self {
            active,
            forks,
            canon,
            generation,
            version: 0,
            journal: VecDeque::new(),
        }
    }

    /// Records of one store, in that store's key order.
    pub fn store(&self, kind: MinerStoreKind) -> &BTreeMap<String, Arc<MinerBlock>> {
        match kind {
            MinerStoreKind::Active => &self.active,
            MinerStoreKind::Forks => &self.forks,
            MinerStoreKind::Canon => &self.canon,
        }
    }

    fn store_mut(&mut self, kind: MinerStoreKind) -> &mut BTreeMap<String, Arc<MinerBlock>> {
        match kind {
            MinerStoreKind::Active => &mut self.active,
            MinerStoreKind::Forks => &mut self.forks,
            MinerStoreKind::Canon => &mut self.canon,
        }
    }

    /// The record a hash lookup returns: MinerActive, then MinerForks, then
    /// MinerCanon, the order `find_by_hash_multi` reads them in.
    pub fn effective(&self, hash: &str) -> Option<&Arc<MinerBlock>> {
        self.active
            .get(hash)
            .or_else(|| self.forks.get(hash))
            .or_else(|| self.canon.get(hash))
    }

    /// Every hash once, with the record `effective` returns for it.
    pub fn effective_all(&self) -> impl Iterator<Item = &Arc<MinerBlock>> + '_ {
        let forks = self
            .forks
            .iter()
            .filter(|(hash, _)| !self.active.contains_key(*hash))
            .map(|(_, block)| block);
        let canon = self
            .canon
            .iter()
            .filter(|(hash, _)| !self.active.contains_key(*hash) && !self.forks.contains_key(*hash))
            .map(|(_, block)| block);
        self.active.values().chain(forks).chain(canon)
    }

    pub fn len_effective(&self) -> usize {
        self.effective_all().count()
    }

    pub(crate) fn put(&mut self, kind: MinerStoreKind, block: &MinerBlock) {
        self.store_mut(kind)
            .insert(block.hash.clone(), Arc::new(block.clone()));
        self.touch(&block.hash);
    }

    pub(crate) fn remove(&mut self, kind: MinerStoreKind, hash: &str) {
        if self.store_mut(kind).remove(hash).is_some() {
            self.touch(hash);
        }
    }

    fn touch(&mut self, hash: &str) {
        self.version += 1;
        self.journal.push_back((self.version, hash.to_string()));
        while self.journal.len() > JOURNAL_CAPACITY {
            self.journal.pop_front();
        }
    }

    /// Identifies this load of the cache. It changes whenever the cache is
    /// rebuilt from the stores, which invalidates every journal position.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// Hashes whose record changed after `version`, each once, in the order
    /// they last changed. `None` when the journal cannot answer: a different
    /// generation, or entries already dropped.
    pub fn changes_since(&self, generation: u64, version: u64) -> Option<Vec<String>> {
        if generation != self.generation || version > self.version {
            return None;
        }
        if version == self.version {
            return Some(Vec::new());
        }
        match self.journal.front() {
            Some((oldest, _)) if *oldest <= version + 1 => {}
            _ => return None,
        }
        let mut seen = std::collections::HashSet::new();
        let mut changed: Vec<String> = self
            .journal
            .iter()
            .rev()
            .take_while(|(at, _)| *at > version)
            .filter(|(_, hash)| seen.insert(hash.as_str()))
            .map(|(_, hash)| hash.clone())
            .collect();
        changed.reverse();
        Some(changed)
    }
}
