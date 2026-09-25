//! Fork choice rules for chain selection.
//!
//! This module implements the canonical fork choice rules used throughout
//! the node for determining which chain to follow when forks occur.
//!
//! The fork choice rules in priority order are:
//! 1. Higher cumulative difficulty wins
//! 2. Higher tip index wins (if difficulty is equal)
//! 3. Lower block hash wins (as final tiebreaker)

use modality_datastore::models::MinerBlock;
use std::collections::BTreeMap;

/// Result of comparing two chains or blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkChoiceResult {
    /// The first/local chain wins
    KeepLocal,
    /// The second/remote chain wins
    AdoptRemote,
    /// Chains are identical
    Equal,
}

/// Detailed comparison information between two chains
#[derive(Debug, Clone)]
pub struct ChainComparison {
    /// The fork choice result
    pub result: ForkChoiceResult,
    /// Local chain cumulative difficulty
    pub local_difficulty: u128,
    /// Remote chain cumulative difficulty
    pub remote_difficulty: u128,
    /// Local chain length
    pub local_length: u64,
    /// Remote chain length
    pub remote_length: u64,
    /// Human-readable reason for the decision
    pub reason: String,
}

/// Compare two chains and determine which one should be canonical.
///
/// Uses fork choice rules in priority order:
/// 1. Higher cumulative difficulty wins
/// 2. Higher tip index wins (if difficulty is equal)
///
/// # Arguments
/// * `local_difficulty` - Cumulative difficulty of the local chain
/// * `local_length` - Local tip index
/// * `remote_difficulty` - Cumulative difficulty of the remote chain
/// * `remote_length` - Remote tip index
///
/// # Returns
/// A `ChainComparison` with the result and reasoning
pub fn compare_chains(
    local_difficulty: u128,
    local_length: u64,
    remote_difficulty: u128,
    remote_length: u64,
) -> ChainComparison {
    let (result, reason) = if remote_difficulty > local_difficulty {
        (
            ForkChoiceResult::AdoptRemote,
            format!(
                "Remote chain has higher cumulative difficulty ({} > {})",
                remote_difficulty, local_difficulty
            ),
        )
    } else if remote_difficulty < local_difficulty {
        (
            ForkChoiceResult::KeepLocal,
            format!(
                "Local chain has higher cumulative difficulty ({} > {})",
                local_difficulty, remote_difficulty
            ),
        )
    } else if remote_length > local_length {
        // Equal difficulty - use length as tiebreaker
        (
            ForkChoiceResult::AdoptRemote,
            format!(
                "Equal difficulty, remote tip is higher ({} > {})",
                remote_length, local_length
            ),
        )
    } else if remote_length < local_length {
        (
            ForkChoiceResult::KeepLocal,
            format!(
                "Equal difficulty, local tip is higher ({} > {})",
                local_length, remote_length
            ),
        )
    } else {
        (
            ForkChoiceResult::Equal,
            "Chains have equal difficulty and length".to_string(),
        )
    };

    ChainComparison {
        result,
        local_difficulty,
        remote_difficulty,
        local_length,
        remote_length,
        reason,
    }
}

/// True when both nodes name the same tip block.
///
/// A hole below that tip changes how much work each node can add up. It does
/// not make one copy of the tip heavier than the other.
pub fn chains_share_tip_hash(local_tip_hash: &str, remote_tip_hash: &str) -> bool {
    !local_tip_hash.is_empty() && local_tip_hash == remote_tip_hash
}

/// Work that fork choice is allowed to compare.
///
/// `Linked` is the sum of a parent-linked walk. `Unknown` is a walk that
/// stopped at a hole or a proof that did not meet its target. Unknown is
/// not zero: it loses to any linked chain, and two unknown chains stay
/// with the local node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainWork {
    Linked(u128),
    Unknown,
}

/// Tip and work of the chain this node should advertise and mine on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredChain {
    pub work: ChainWork,
    pub tip: u64,
    pub tip_hash: String,
}

/// Whether a fetched peer chain replaces the local one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    Adopt { ancestor_index: u64, reason: String },
    Refuse { reason: String },
}

/// Choose which chain to follow from two linked totals.
///
/// The same tip hash is equal work. A higher tip does not beat more
/// verified work.
pub fn choose_chain(
    local_difficulty: u128,
    local_tip: u64,
    local_tip_hash: &str,
    remote_difficulty: u128,
    remote_tip: u64,
    remote_tip_hash: &str,
) -> ChainComparison {
    choose_verified(
        ChainWork::Linked(local_difficulty),
        local_tip,
        local_tip_hash,
        ChainWork::Linked(remote_difficulty),
        remote_tip,
        remote_tip_hash,
    )
}

/// Choose which chain to follow when one or both totals may be unknown.
pub fn choose_verified(
    local: ChainWork,
    local_tip: u64,
    local_tip_hash: &str,
    remote: ChainWork,
    remote_tip: u64,
    remote_tip_hash: &str,
) -> ChainComparison {
    let local_difficulty = match local {
        ChainWork::Linked(work) => work,
        ChainWork::Unknown => 0,
    };
    let remote_difficulty = match remote {
        ChainWork::Linked(work) => work,
        ChainWork::Unknown => 0,
    };
    if chains_share_tip_hash(local_tip_hash, remote_tip_hash) {
        return ChainComparison {
            result: ForkChoiceResult::Equal,
            local_difficulty,
            remote_difficulty,
            local_length: local_tip,
            remote_length: remote_tip,
            reason: format!("Same tip hash {local_tip_hash}, equal work"),
        };
    }
    let (result, reason) = match (local, remote) {
        (ChainWork::Unknown, ChainWork::Unknown) => (
            ForkChoiceResult::KeepLocal,
            "Neither chain's work reaches its anchor".to_string(),
        ),
        (ChainWork::Unknown, ChainWork::Linked(work)) => (
            ForkChoiceResult::AdoptRemote,
            format!("Peer work {work} is linked and local work stopped at a hole"),
        ),
        (ChainWork::Linked(work), ChainWork::Unknown) => (
            ForkChoiceResult::KeepLocal,
            format!("Local work {work} is linked and peer work stopped at a hole"),
        ),
        (ChainWork::Linked(local_work), ChainWork::Linked(remote_work))
            if remote_work > local_work =>
        {
            (
                ForkChoiceResult::AdoptRemote,
                format!("Peer linked work {remote_work} is higher than local {local_work}"),
            )
        }
        (ChainWork::Linked(local_work), ChainWork::Linked(remote_work))
            if remote_work < local_work =>
        {
            (
                ForkChoiceResult::KeepLocal,
                format!("Local linked work {local_work} is higher than peer {remote_work}"),
            )
        }
        (ChainWork::Linked(_), ChainWork::Linked(_)) if remote_tip > local_tip => (
            ForkChoiceResult::AdoptRemote,
            format!("Equal linked work, peer tip {remote_tip} is higher than {local_tip}"),
        ),
        (ChainWork::Linked(_), ChainWork::Linked(_)) if remote_tip < local_tip => (
            ForkChoiceResult::KeepLocal,
            format!("Equal linked work, local tip {local_tip} is higher than {remote_tip}"),
        ),
        (ChainWork::Linked(_), ChainWork::Linked(_)) if remote_tip_hash < local_tip_hash => (
            ForkChoiceResult::AdoptRemote,
            "Equal linked work and tip, peer hash is lower".to_string(),
        ),
        (ChainWork::Linked(_), ChainWork::Linked(_)) if remote_tip_hash > local_tip_hash => (
            ForkChoiceResult::KeepLocal,
            "Equal linked work and tip, local hash is lower".to_string(),
        ),
        _ => (
            ForkChoiceResult::Equal,
            "Chains have equal linked work, tip, and hash".to_string(),
        ),
    };
    ChainComparison {
        result,
        local_difficulty,
        remote_difficulty,
        local_length: local_tip,
        remote_length: remote_tip,
        reason,
    }
}

/// Network parameters the miner already uses to retarget.
#[derive(Debug, Clone, Copy)]
pub struct RetargetParams {
    pub blocks_per_epoch: u64,
    pub target_block_time_secs: u64,
    pub initial_difficulty: Option<u128>,
}

/// Whether a block's target is the one this chain expects at its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetCheck {
    /// The target matches the retarget, or the same-epoch parent.
    Matches,
    /// The target is not the one the previous span requires.
    Reject,
    /// The previous span is not stored yet, so the retarget cannot be checked.
    Unresolved,
}

/// Check the target against the miner epoch retarget.
///
/// Inside an epoch the target has to match the parent. At an epoch
/// boundary it has to match `EpochManager::get_difficulty_for_block`
/// for the previous span. A span that is not stored yet is unresolved,
/// not a pass.
pub fn check_expected_target(
    block: &MinerBlock,
    stored: &[MinerBlock],
    params: RetargetParams,
) -> TargetCheck {
    let Ok(target) = block.target_difficulty.parse::<u128>() else {
        return TargetCheck::Reject;
    };
    if target == 0 {
        return TargetCheck::Reject;
    }
    let blocks_per_epoch = params.blocks_per_epoch.max(1);
    let manager = modality_miner::EpochManager::new(
        blocks_per_epoch,
        params.target_block_time_secs.max(1),
        params.initial_difficulty.unwrap_or(1000),
    );
    let epoch = manager.get_epoch(block.index);
    if epoch == 0 {
        return match params.initial_difficulty {
            Some(initial) if target == initial => TargetCheck::Matches,
            Some(_) => TargetCheck::Reject,
            None => TargetCheck::Unresolved,
        };
    }
    let ancestors = ancestors_of(block, stored);
    if let Some(parent) = ancestors.last() {
        if manager.get_epoch(parent.index) == epoch
            && parent.target_difficulty != block.target_difficulty
        {
            return TargetCheck::Reject;
        }
    }
    let prev_epoch = epoch - 1;
    let start = manager.get_epoch_start_index(prev_epoch);
    let end = manager.get_epoch_end_index(prev_epoch);
    let mut span: Vec<&MinerBlock> = ancestors
        .iter()
        .copied()
        .filter(|ancestor| ancestor.index >= start && ancestor.index <= end)
        .collect();
    span.sort_by_key(|ancestor| ancestor.index);
    span.dedup_by_key(|ancestor| ancestor.index);
    if span.len() != blocks_per_epoch as usize {
        return TargetCheck::Unresolved;
    }
    let chain = span
        .iter()
        .filter_map(|ancestor| miner_block_as_chain_block(ancestor))
        .collect::<Vec<_>>();
    if chain.len() != span.len() {
        return TargetCheck::Unresolved;
    }
    let expected = manager.get_difficulty_for_block(block.index, &chain);
    if target == expected {
        TargetCheck::Matches
    } else {
        TargetCheck::Reject
    }
}

fn ancestors_of<'a>(block: &MinerBlock, stored: &'a [MinerBlock]) -> Vec<&'a MinerBlock> {
    let mut by_hash: std::collections::HashMap<&str, &MinerBlock> =
        std::collections::HashMap::new();
    for stored_block in stored {
        by_hash
            .entry(stored_block.hash.as_str())
            .or_insert(stored_block);
    }
    let mut chain = Vec::new();
    let mut previous = block.previous_hash.as_str();
    let mut index = block.index;
    while index > 0 {
        let Some(parent) = by_hash.get(previous).copied() else {
            break;
        };
        if parent.index + 1 != index {
            break;
        }
        chain.push(parent);
        previous = parent.previous_hash.as_str();
        index = parent.index;
    }
    chain.reverse();
    chain
}

fn miner_block_as_chain_block(block: &MinerBlock) -> Option<modality_miner::Block> {
    let difficulty = block.target_difficulty.parse::<u128>().ok()?;
    let nonce = block.nonce.parse::<u128>().unwrap_or(0);
    let timestamp = chrono::DateTime::from_timestamp(block.timestamp, 0)?;
    Some(modality_miner::Block {
        header: modality_miner::BlockHeader {
            index: block.index,
            timestamp,
            previous_hash: block.previous_hash.clone(),
            data_hash: block.data_hash.clone(),
            nonce,
            difficulty,
            hash: block.hash.clone(),
        },
        data: modality_miner::BlockData {
            nominated_peer_id: block.nominated_peer_id.clone(),
            miner_number: block.miner_number,
        },
    })
}

/// True when the block's actualized difficulty meets the target it claims.
pub fn proof_meets_target(block: &MinerBlock) -> bool {
    let Ok(actual) = block.get_actualized_difficulty_u128() else {
        return false;
    };
    let Ok(target) = block.target_difficulty.parse::<u128>() else {
        return false;
    };
    target > 0 && actual >= target
}

/// Score the canonical set. A walk that reaches index 0 is linked work.
/// Anything else is unknown, and the tip is that fallback spine.
pub fn score_canonical_chain(blocks: &[MinerBlock]) -> ScoredChain {
    let spine = MinerBlock::verified_spine(blocks);
    let Some(tip) = spine.last() else {
        return ScoredChain {
            work: ChainWork::Unknown,
            tip: 0,
            tip_hash: String::new(),
        };
    };
    let reaches_genesis = spine.first().is_some_and(|block| block.index == 0);
    if !reaches_genesis {
        return ScoredChain {
            work: ChainWork::Unknown,
            tip: tip.index,
            tip_hash: tip.hash.clone(),
        };
    }
    let mut sum = 0u128;
    for block in &spine {
        if !proof_meets_target(block) {
            return ScoredChain {
                work: ChainWork::Unknown,
                tip: tip.index,
                tip_hash: tip.hash.clone(),
            };
        }
        let Ok(difficulty) = block.get_actualized_difficulty_u128() else {
            return ScoredChain {
                work: ChainWork::Unknown,
                tip: tip.index,
                tip_hash: tip.hash.clone(),
            };
        };
        sum = sum.saturating_add(difficulty);
    }
    ScoredChain {
        work: ChainWork::Linked(sum),
        tip: tip.index,
        tip_hash: tip.hash.clone(),
    }
}

/// Sum of a parent-linked batch. One block that misses its target makes
/// the whole suffix unknown.
pub fn verified_suffix_work(blocks: &[MinerBlock]) -> ChainWork {
    if blocks.is_empty() {
        return ChainWork::Unknown;
    }
    let mut sum = 0u128;
    for block in blocks {
        if !proof_meets_target(block) {
            return ChainWork::Unknown;
        }
        let Ok(difficulty) = block.get_actualized_difficulty_u128() else {
            return ChainWork::Unknown;
        };
        sum = sum.saturating_add(difficulty);
    }
    ChainWork::Linked(sum)
}

/// True when the linked spine has every block of the nomination epoch.
///
/// An epoch below 2 has no nomination yet, so this does not block adoption.
pub fn nomination_epoch_complete(
    blocks: &[MinerBlock],
    blocks_per_epoch: u64,
    mining_epoch: u64,
) -> bool {
    if mining_epoch < 2 || blocks_per_epoch == 0 {
        return true;
    }
    let nomination_epoch = mining_epoch - 2;
    let spine = MinerBlock::verified_spine(blocks);
    let start = nomination_epoch.saturating_mul(blocks_per_epoch);
    let end = start + blocks_per_epoch;
    let count = spine
        .iter()
        .filter(|block| block.index >= start && block.index < end)
        .count();
    count == blocks_per_epoch as usize
}

/// Decide whether a validated peer chain replaces the local canonical set.
///
/// `checkpoint_floor` is the last miner index the sequencers have committed.
/// An adoption that would orphan that prefix is refused. A peer chain that
/// drops a nomination epoch the local chain already has is refused.
pub fn decide_adoption(
    local_blocks: &[MinerBlock],
    remote_blocks: &[MinerBlock],
    checkpoint_floor: Option<u64>,
    blocks_per_epoch: u64,
) -> Adoption {
    let Some(first) = remote_blocks.first() else {
        return Adoption::Refuse {
            reason: "Peer batch is empty".to_string(),
        };
    };
    let linked = remote_blocks
        .windows(2)
        .all(|pair| pair[1].index == pair[0].index + 1 && pair[1].previous_hash == pair[0].hash);
    if !linked {
        return Adoption::Refuse {
            reason: "Peer batch is not one parent-linked chain".to_string(),
        };
    }
    if first.index == 0 {
        if !local_blocks.is_empty() {
            return Adoption::Refuse {
                reason: "No common ancestor with the local chain".to_string(),
            };
        }
    } else if !local_blocks
        .iter()
        .any(|block| block.index == first.index - 1 && block.hash == first.previous_hash)
    {
        return Adoption::Refuse {
            reason: format!(
                "Peer chain does not link to a stored block at {}",
                first.index - 1
            ),
        };
    }

    let ancestor_index = first.index.saturating_sub(1);
    let remote_tip = remote_blocks.last().expect("remote batch is non-empty");
    let remote_work = verified_suffix_work(remote_blocks);
    let (local_work, local_tip, local_hash) = if first.index == 0 || local_blocks.is_empty() {
        (ChainWork::Unknown, 0, String::new())
    } else {
        work_above_ancestor(local_blocks, ancestor_index, &first.previous_hash)
    };

    if let Some(floor) = checkpoint_floor {
        if first.index > 0
            && local_blocks
                .iter()
                .any(|block| block.index > ancestor_index && block.index <= floor)
        {
            return Adoption::Refuse {
                reason: format!("Refusing to orphan the sequenced prefix through {floor}"),
            };
        }
    }

    let local_epoch = MinerBlock::verified_spine(local_blocks)
        .last()
        .map(|block| block.epoch)
        .unwrap_or(0);
    let mut candidate: Vec<MinerBlock> = local_blocks
        .iter()
        .filter(|block| first.index == 0 || block.index <= ancestor_index)
        .cloned()
        .collect();
    candidate.extend(remote_blocks.iter().cloned());
    if nomination_epoch_complete(local_blocks, blocks_per_epoch, local_epoch)
        && !nomination_epoch_complete(&candidate, blocks_per_epoch, remote_tip.epoch)
    {
        return Adoption::Refuse {
            reason: "Peer chain does not cover the nomination epoch the local chain already has"
                .to_string(),
        };
    }

    let comparison = choose_verified(
        local_work,
        local_tip,
        &local_hash,
        remote_work,
        remote_tip.index,
        &remote_tip.hash,
    );
    if comparison.result != ForkChoiceResult::AdoptRemote {
        return Adoption::Refuse {
            reason: comparison.reason,
        };
    }
    Adoption::Adopt {
        ancestor_index,
        reason: comparison.reason,
    }
}

fn work_above_ancestor(
    blocks: &[MinerBlock],
    ancestor_index: u64,
    ancestor_hash: &str,
) -> (ChainWork, u64, String) {
    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let mut best: Option<(u128, u64, String)> = None;
    for block in blocks.iter().filter(|block| block.index > ancestor_index) {
        let mut sum = 0u128;
        let mut current = block;
        let mut reached = false;
        loop {
            if !proof_meets_target(current) {
                break;
            }
            let Ok(difficulty) = current.get_actualized_difficulty_u128() else {
                break;
            };
            sum = sum.saturating_add(difficulty);
            if current.index == ancestor_index + 1 {
                reached = current.previous_hash == ancestor_hash;
                break;
            }
            let Some(parents) = by_index.get(&(current.index - 1)) else {
                break;
            };
            let Some(parent) = parents
                .iter()
                .copied()
                .find(|parent| parent.hash == current.previous_hash)
            else {
                break;
            };
            current = parent;
        }
        if !reached {
            continue;
        }
        let better = match &best {
            None => true,
            Some((work, tip, hash)) => {
                sum > *work
                    || (sum == *work && block.index > *tip)
                    || (sum == *work && block.index == *tip && block.hash < *hash)
            }
        };
        if better {
            best = Some((sum, block.index, block.hash.clone()));
        }
    }
    match best {
        Some((sum, tip, hash)) => (ChainWork::Linked(sum), tip, hash),
        None => {
            let tip = blocks.iter().max_by_key(|block| block.index);
            (
                ChainWork::Unknown,
                tip.map(|block| block.index).unwrap_or(0),
                tip.map(|block| block.hash.clone()).unwrap_or_default(),
            )
        }
    }
}

/// Compare two blocks at the same height for fork choice.
///
/// Higher verified difficulty wins. Equal verified difficulty keeps the
/// block already stored. A difficulty that cannot be read does not win.
pub fn should_replace_block(new_block: &MinerBlock, existing_block: &MinerBlock) -> bool {
    compare_blocks(new_block, existing_block).should_replace
}

/// Detailed fork choice result for block comparison
#[derive(Debug, Clone)]
pub struct BlockForkChoiceResult {
    /// Whether to replace the existing block
    pub should_replace: bool,
    /// New block difficulty
    pub new_difficulty: u128,
    /// Existing block difficulty
    pub existing_difficulty: u128,
    /// Human-readable reason
    pub reason: String,
}

/// Compare two blocks and return detailed fork choice result.
///
/// # Arguments
/// * `new_block` - The new/incoming block
/// * `existing_block` - The existing canonical block
///
/// # Returns
/// Detailed fork choice result with reasoning
pub fn compare_blocks(
    new_block: &MinerBlock,
    existing_block: &MinerBlock,
) -> BlockForkChoiceResult {
    let new_difficulty = new_block.get_actualized_difficulty_u128().ok();
    let existing_difficulty = existing_block.get_actualized_difficulty_u128().ok();
    let (should_replace, reason) = match (new_difficulty, existing_difficulty) {
        (Some(new_work), Some(existing_work))
            if new_work > existing_work && proof_meets_target(new_block) =>
        {
            (
                true,
                format!("Higher actualized difficulty ({new_work} > {existing_work})"),
            )
        }
        (Some(new_work), Some(existing_work)) if new_work > existing_work => (
            false,
            format!("Higher actualized difficulty ({new_work}) does not meet its target"),
        ),
        (Some(new_work), Some(existing_work)) if new_work < existing_work => (
            false,
            format!("Lower actualized difficulty ({new_work} < {existing_work})"),
        ),
        (Some(_), Some(_)) => (
            false,
            "Equal verified difficulty keeps the stored block".to_string(),
        ),
        (None, _) => (false, "Incoming difficulty cannot be read".to_string()),
        (Some(new_work), None) if proof_meets_target(new_block) => (
            true,
            format!("Stored difficulty cannot be read and incoming work is {new_work}"),
        ),
        (Some(_), None) => (
            false,
            "Incoming difficulty does not meet its target".to_string(),
        ),
    };
    BlockForkChoiceResult {
        should_replace,
        new_difficulty: new_difficulty.unwrap_or(0),
        existing_difficulty: existing_difficulty.unwrap_or(0),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_chains_difficulty_wins() {
        let result = compare_chains(100, 10, 200, 5);
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);

        let result = compare_chains(200, 5, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn test_compare_chains_length_tiebreaker() {
        let result = compare_chains(100, 10, 100, 15);
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);

        let result = compare_chains(100, 15, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn test_compare_chains_equal() {
        let result = compare_chains(100, 10, 100, 10);
        assert_eq!(result.result, ForkChoiceResult::Equal);
    }

    #[test]
    fn same_tip_hash_is_equal_work_despite_a_hole() {
        let result = choose_chain(3348, 1083, "abc", 3307, 1083, "abc");
        assert_eq!(result.result, ForkChoiceResult::Equal);
    }

    #[test]
    fn higher_peer_tip_does_not_beat_more_linked_work() {
        let result = choose_chain(3348, 1080, "local", 3307, 1083, "remote");
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn unknown_work_loses_to_a_linked_chain() {
        let result = choose_verified(
            ChainWork::Unknown,
            500,
            "local",
            ChainWork::Linked(10),
            20,
            "remote",
        );
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);
        let result = choose_verified(
            ChainWork::Linked(10),
            20,
            "local",
            ChainWork::Unknown,
            500,
            "remote",
        );
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn two_unknown_chains_stay_local() {
        let result = choose_verified(
            ChainWork::Unknown,
            10,
            "local",
            ChainWork::Unknown,
            800,
            "remote",
        );
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }

    #[test]
    fn equal_linked_work_prefers_the_higher_tip() {
        let result = choose_verified(
            ChainWork::Linked(100),
            4,
            "local",
            ChainWork::Linked(100),
            6,
            "remote",
        );
        assert_eq!(result.result, ForkChoiceResult::AdoptRemote);
    }

    fn block_at(index: u64, prev: &str, work: &str) -> MinerBlock {
        let mut block = MinerBlock::new_canonical(
            format!("hash_{index}"),
            index,
            0,
            1_700_000_000 + index as i64,
            prev.to_string(),
            format!("data_{index}"),
            1,
            1000,
            "peer".to_string(),
            1,
        );
        block.actualized_difficulty = work.to_string();
        block.target_difficulty = "1000".to_string();
        block
    }

    #[test]
    fn unreadable_difficulty_does_not_replace_the_stored_block() {
        let existing = block_at(1, "hash_0", "2000");
        let mut incoming = block_at(1, "hash_0", "9999");
        incoming.hash = "other".to_string();
        incoming.actualized_difficulty = "nope".to_string();
        incoming.seen_at = Some(1);
        existing_keeps(&existing, &incoming);
    }

    fn existing_keeps(existing: &MinerBlock, incoming: &MinerBlock) {
        assert!(!should_replace_block(incoming, existing));
    }

    #[test]
    fn equal_work_keeps_the_stored_block() {
        let mut existing = block_at(1, "hash_0", "2000");
        existing.seen_at = Some(100);
        let mut incoming = block_at(1, "hash_0", "2000");
        incoming.hash = "aaa".to_string();
        incoming.seen_at = Some(1);
        assert!(!should_replace_block(&incoming, &existing));
    }

    #[test]
    fn lower_index_with_more_verified_work_wins() {
        let local = vec![
            block_at(0, "genesis", "1000"),
            block_at(1, "hash_0", "1000"),
            block_at(2, "hash_1", "1000"),
        ];
        let mut remote = block_at(1, "hash_0", "5000");
        remote.hash = "heavier".to_string();
        match decide_adoption(&local, &[remote], None, 0) {
            Adoption::Adopt { ancestor_index, .. } => assert_eq!(ancestor_index, 0),
            Adoption::Refuse { reason } => panic!("expected adopt, got {reason}"),
        }
    }

    #[test]
    fn adopt_does_not_orphan_a_checkpoint_prefix() {
        let local = vec![
            block_at(0, "genesis", "1000"),
            block_at(1, "hash_0", "1000"),
            block_at(2, "hash_1", "1000"),
            block_at(3, "hash_2", "1000"),
            block_at(4, "hash_3", "1000"),
        ];
        let remote = vec![block_at(3, "hash_2", "9000"), {
            let mut block = block_at(4, "hash_3", "9000");
            block.hash = "alt_4".to_string();
            block.previous_hash = "hash_3".to_string();
            block
        }];
        match decide_adoption(&local, &remote, Some(4), 0) {
            Adoption::Refuse { reason } => assert!(reason.contains("sequenced prefix")),
            Adoption::Adopt { .. } => panic!("checkpoint prefix was orphaned"),
        }
    }

    #[test]
    fn a_hole_does_not_make_the_higher_index_the_verified_tip() {
        let blocks = vec![
            block_at(0, "genesis", "1000"),
            block_at(1, "hash_0", "1000"),
            block_at(4, "hash_3", "1000"),
            block_at(5, "hash_4", "1000"),
        ];
        let scored = score_canonical_chain(&blocks);
        assert_eq!(scored.tip, 1);
        assert!(matches!(scored.work, ChainWork::Linked(_)));
    }

    #[test]
    fn gapped_local_suffix_loses_to_a_linked_peer_block() {
        let local = vec![
            block_at(0, "genesis", "1000"),
            block_at(8, "missing", "1000"),
            block_at(9, "hash_8", "1000"),
        ];
        let remote = vec![block_at(1, "hash_0", "1000")];
        match decide_adoption(&local, &remote, None, 0) {
            Adoption::Adopt { ancestor_index, .. } => assert_eq!(ancestor_index, 0),
            Adoption::Refuse { reason } => panic!("linked peer block should win: {reason}"),
        }
    }

    #[test]
    fn incomplete_nomination_epoch_does_not_replace_a_complete_one() {
        let mut local = vec![
            block_at(0, "genesis", "1000"),
            block_at(1, "hash_0", "1000"),
            block_at(2, "hash_1", "1000"),
        ];
        local[2].epoch = 2;
        let mut remote = block_at(3, "hash_2", "9000");
        remote.hash = "alt_3".to_string();
        remote.epoch = 4;
        let mut remote_next = block_at(4, "alt_3", "9000");
        remote_next.hash = "alt_4".to_string();
        remote_next.previous_hash = "alt_3".to_string();
        remote_next.epoch = 4;
        match decide_adoption(&local, &[remote, remote_next], None, 2) {
            Adoption::Refuse { reason } => assert!(reason.contains("nomination")),
            Adoption::Adopt { .. } => panic!("incomplete nomination replaced a complete chain"),
        }
    }

    #[test]
    fn a_new_epoch_cannot_name_its_own_target() {
        let mut first = block_at(1, "hash_0", "1000");
        first.timestamp = 1_000;
        let mut second = block_at(2, "hash_1", "1000");
        second.timestamp = 1_001;
        let stored = vec![block_at(0, "genesis", "1000"), first, second];
        let params = RetargetParams {
            blocks_per_epoch: 2,
            target_block_time_secs: 60,
            initial_difficulty: Some(1000),
        };
        let mut cheap = block_at(3, "hash_2", "1000");
        cheap.target_difficulty = "1000".to_string();
        assert_eq!(
            check_expected_target(&cheap, &stored, params),
            TargetCheck::Reject
        );
        let mut retargeted = cheap.clone();
        retargeted.target_difficulty = "8000".to_string();
        retargeted.actualized_difficulty = "8000".to_string();
        assert_eq!(
            check_expected_target(&retargeted, &stored, params),
            TargetCheck::Matches
        );
    }

    #[test]
    fn lower_peer_tip_still_loses_on_difficulty() {
        let result = choose_chain(3348, 1083, "local", 3307, 1080, "remote");
        assert_eq!(result.result, ForkChoiceResult::KeepLocal);
    }
}
