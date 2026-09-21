//! Structured node status for the HTTP page and the terminal UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use libp2p::Multiaddr;
use modality_datastore::models::miner::MinerBlock;
use modality_datastore::models::validator::{
    get_validator_set_for_mining_epoch_hybrid_multi, ValidatorBlock,
};
use modality_datastore::DatastoreManager;

use crate::constants::{
    BFT_THRESHOLD_PERCENTAGE, NETWORK_HASHRATE_SAMPLE_SIZE, STATUS_EPOCHS_TO_SHOW,
    STATUS_FINALIZED_ROUNDS_TO_SHOW, STATUS_FIRST_BLOCKS_COUNT, STATUS_RECENT_BLOCKS_COUNT,
    STATUS_RECENT_PREFIX_CERTS_COUNT,
};
use crate::mining_metrics::SharedMiningMetrics;

/// Handles the TUI (and other live viewers) need to poll a running node.
#[derive(Clone)]
pub struct NodeStatusSource {
    pub peerid: libp2p_identity::PeerId,
    pub role: String,
    pub network_name: String,
    pub listeners: Vec<Multiaddr>,
    pub status_port: Option<u16>,
    pub hybrid_consensus: bool,
    pub run_miner: bool,
    pub run_validator: bool,
    pub run_contract_validator: bool,
    pub datastore: Arc<Mutex<DatastoreManager>>,
    pub swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    pub mining_metrics: SharedMiningMetrics,
    pub shutdown_tx: broadcast::Sender<()>,
    pub mining_shutdown: Option<Arc<AtomicBool>>,
}

impl NodeStatusSource {
    /// Ask the node to stop mining loops and exit `wait_for_shutdown`.
    pub fn request_shutdown(&self) {
        modality_common::hash_tax::set_mining_shutdown(true);
        if let Some(ref flag) = self.mining_shutdown {
            flag.store(true, Ordering::Relaxed);
        }
        let _ = self.shutdown_tx.send(());
    }

    pub async fn snapshot(&self) -> anyhow::Result<NodeStatus> {
        collect_node_status(self).await
    }
}

#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub peerid: String,
    pub role: String,
    pub role_display: String,
    pub active_roles: Vec<&'static str>,
    pub network_name: String,
    pub hybrid_consensus: bool,
    pub listeners: Vec<String>,
    pub status_url: Option<String>,
    pub connected_peers: usize,
    pub peers: Vec<PeerStatus>,
    pub chain_tip: u64,
    pub current_epoch: u64,
    pub blocks_per_epoch: u64,
    pub total_miner_blocks: usize,
    pub blocks_mined_by_node: usize,
    pub current_difficulty: String,
    pub cumulative_difficulty: u128,
    pub miner_hashrate: String,
    pub network_hashrate: String,
    pub current_round: u64,
    pub genesis: Option<GenesisBlock>,
    pub recent_blocks: Vec<BlockStatus>,
    pub first_blocks: Vec<BlockStatus>,
    pub finalized_rounds: Vec<RoundStatus>,
    pub epoch_nominees: Vec<EpochNominees>,
    pub sequencer_committee: Vec<String>,
    pub sequencer_nomination_epoch: Option<u64>,
    pub named_validators: Vec<String>,
    pub validator_min_stake: u64,
    pub validator_qc_numerator: u64,
    pub validator_qc_denominator: u64,
    pub dest_apply_requires_cert: bool,
    pub recent_prefix_certs: Vec<PrefixCertStatus>,
    pub pending_prefix_cert_requests: usize,
}

#[derive(Debug, Clone)]
pub struct PeerStatus {
    pub peer_id: String,
    pub role: Option<String>,
    pub status_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlockStatus {
    pub index: u64,
    pub epoch: u64,
    pub hash: String,
    pub nominee: String,
    pub timestamp: i64,
    pub time_delta: String,
}

#[derive(Debug, Clone)]
pub struct GenesisBlock {
    pub index: u64,
    pub hash: String,
    pub epoch: u64,
    pub timestamp: i64,
    pub previous_hash: String,
    pub data_hash: String,
    pub difficulty: String,
    pub nominated_peer_id: String,
}

#[derive(Debug, Clone)]
pub struct RoundStatus {
    pub round_id: u64,
    pub certified_count: usize,
    pub total_count: usize,
    pub status: &'static str,
}

#[derive(Debug, Clone)]
pub struct EpochNominee {
    pub rank: usize,
    pub block_index: u64,
    pub block_hash: String,
    pub peer_id: String,
}

#[derive(Debug, Clone)]
pub struct EpochNominees {
    pub epoch: u64,
    pub nominees: Vec<EpochNominee>,
}

#[derive(Debug, Clone)]
pub struct PrefixCertStatus {
    pub source_contract: String,
    pub through_commit: String,
    pub validator_peer_id: String,
    pub prefix_digest: String,
}

impl NodeStatus {
    pub fn epoch_progress(&self) -> f64 {
        if self.blocks_per_epoch == 0 {
            return 0.0;
        }
        (self.chain_tip % self.blocks_per_epoch) as f64 / self.blocks_per_epoch as f64
    }

    pub fn hybrid_label(&self) -> &'static str {
        if self.hybrid_consensus {
            "on (sequencers from epoch N−2)"
        } else {
            "off"
        }
    }
}

/// Join protocol role chips for status display, e.g. `Miner+Sequencer+Validator`.
pub fn format_enumerated_roles(roles: &[&str]) -> String {
    roles.join("+")
}

/// Map stored / gossip role strings onto enumerated protocol role names.
pub fn display_node_role(role: &str) -> String {
    let roles = derive_active_roles(role, false, false, false, false, &[], "");
    if !roles.is_empty() {
        return format_enumerated_roles(&roles);
    }
    match role.trim().to_ascii_lowercase().as_str() {
        "observer" => "Observer".to_string(),
        "noop" => "Noop".to_string(),
        "" => "Unknown".to_string(),
        other => other.to_string(),
    }
}

/// Protocol role chips for this process. A node may run one, two, or all three.
pub fn derive_active_roles(
    role: &str,
    run_miner: bool,
    run_validator: bool,
    run_contract_validator: bool,
    hybrid_consensus: bool,
    named_validators: &[String],
    peerid: &str,
) -> Vec<&'static str> {
    let r = role.trim().to_ascii_lowercase();
    let miner = run_miner
        || matches!(
            r.as_str(),
            "miner" | "hybrid" | "miner+validator" | "miner+sequencer"
        );
    let sequencer = run_validator
        || matches!(
            r.as_str(),
            "sequencer" | "validator" | "hybrid" | "miner+validator" | "miner+sequencer"
        )
        || (hybrid_consensus && miner);
    let validator = run_contract_validator
        || named_validators.iter().any(|id| id == peerid)
        || matches!(r.as_str(), "contract-validator" | "contract_validator");
    let mut out = Vec::new();
    if miner {
        out.push("Miner");
    }
    if sequencer {
        out.push("Sequencer");
    }
    if validator {
        out.push("Validator");
    }
    out
}

/// Collect a live status snapshot from node handles.
pub async fn collect_node_status(source: &NodeStatusSource) -> anyhow::Result<NodeStatus> {
    let peer_ids = {
        let swarm = source.swarm.lock().await;
        swarm.connected_peers().cloned().collect::<Vec<_>>()
    };

    let mgr = source.datastore.lock().await;
    let blocks_per_epoch = mgr.epoch_config().blocks_per_epoch.max(1);
    let current_round = mgr.get_current_round().await.unwrap_or(0);
    let miner_blocks = MinerBlock::find_all_canonical_multi(&mgr)
        .await
        .unwrap_or_default();

    let latest_block = miner_blocks.iter().max_by_key(|b| b.index);
    let current_difficulty = latest_block
        .map(|b| b.target_difficulty.clone())
        .unwrap_or_else(|| "0".to_string());
    let current_epoch = latest_block.map(|b| b.epoch).unwrap_or(0);
    let chain_tip = latest_block.map(|b| b.index).unwrap_or(0);

    let peerid_str = source.peerid.to_string();
    let blocks_mined_by_node = miner_blocks
        .iter()
        .filter(|block| block.nominated_peer_id == peerid_str)
        .count();

    let cumulative_difficulty: u128 = miner_blocks
        .iter()
        .filter_map(|block| block.target_difficulty.parse::<u128>().ok())
        .sum();

    let network_hashrate = calculate_network_hashrate(&miner_blocks);
    let miner_hashrate = {
        let metrics = source.mining_metrics.read().await;
        metrics.average_hashrate()
    };

    let block_map: std::collections::HashMap<u64, &MinerBlock> = miner_blocks
        .iter()
        .map(|block| (block.index, block))
        .collect();

    let mut recent_blocks: Vec<BlockStatus> = miner_blocks
        .iter()
        .map(|b| block_status(b, &block_map))
        .collect();
    recent_blocks.sort_by_key(|b| std::cmp::Reverse(b.index));
    recent_blocks.truncate(STATUS_RECENT_BLOCKS_COUNT);

    let mut first_blocks: Vec<BlockStatus> = miner_blocks
        .iter()
        .map(|b| block_status(b, &block_map))
        .collect();
    first_blocks.sort_by_key(|b| b.index);
    first_blocks.truncate(STATUS_FIRST_BLOCKS_COUNT);

    let genesis = miner_blocks
        .iter()
        .find(|b| b.index == 0)
        .map(|block| GenesisBlock {
            index: block.index,
            hash: block.hash.clone(),
            epoch: block.epoch,
            timestamp: block.timestamp,
            previous_hash: block.previous_hash.clone(),
            data_hash: block.data_hash.clone(),
            difficulty: block.target_difficulty.clone(),
            nominated_peer_id: block.nominated_peer_id.clone(),
        });

    let mut peers = Vec::new();
    for peer_id in &peer_ids {
        let peer_id_str = peer_id.to_string();
        let info = modality_datastore::models::PeerInfo::find_one(&mgr, &peer_id_str)
            .await
            .ok()
            .flatten();
        peers.push(PeerStatus {
            peer_id: peer_id_str,
            role: info.as_ref().and_then(|i| i.role.clone()),
            status_url: info.and_then(|i| i.status_url),
        });
    }

    let finalized_rounds = collect_finalized_rounds(&mgr, current_round).await;
    let epoch_nominees = collect_epoch_nominees(&miner_blocks, current_epoch, blocks_per_epoch);

    let (sequencer_committee, sequencer_nomination_epoch) =
        if source.hybrid_consensus && current_epoch >= 2 {
            match get_validator_set_for_mining_epoch_hybrid_multi(&mgr, current_epoch).await {
                Ok(Some(set)) => (
                    set.get_active_validators(),
                    Some(current_epoch.saturating_sub(2)),
                ),
                _ => (Vec::new(), Some(current_epoch.saturating_sub(2))),
            }
        } else {
            (Vec::new(), None)
        };

    let named_validators = mgr.contract_validators().unwrap_or_default();
    let validator_min_stake = mgr.validator_min_stake().unwrap_or(0);
    let validator_qc_numerator = mgr.validator_qc_numerator().unwrap_or(2);
    let validator_qc_denominator = mgr.validator_qc_denominator().unwrap_or(3);
    let dest_apply_requires_cert = mgr.dest_apply_requires_validator_cert().unwrap_or(false);
    let recent_prefix_certs = mgr
        .list_recent_prefix_certs(STATUS_RECENT_PREFIX_CERTS_COUNT)
        .unwrap_or_default()
        .iter()
        .map(prefix_cert_status)
        .collect();
    let pending_prefix_cert_requests = mgr
        .peek_prefix_cert_requests()
        .map(|r| r.len())
        .unwrap_or(0);

    let active_roles = derive_active_roles(
        &source.role,
        source.run_miner,
        source.run_validator,
        source.run_contract_validator,
        source.hybrid_consensus,
        &named_validators,
        &peerid_str,
    );
    drop(mgr);

    Ok(NodeStatus {
        peerid: peerid_str,
        role: source.role.clone(),
        role_display: if active_roles.is_empty() {
            display_node_role(&source.role)
        } else {
            format_enumerated_roles(&active_roles)
        },
        active_roles,
        network_name: source.network_name.clone(),
        hybrid_consensus: source.hybrid_consensus,
        listeners: source.listeners.iter().map(|l| l.to_string()).collect(),
        status_url: source.status_port.map(|p| format!("http://localhost:{p}")),
        connected_peers: peer_ids.len(),
        peers,
        chain_tip,
        current_epoch,
        blocks_per_epoch,
        total_miner_blocks: miner_blocks.len(),
        blocks_mined_by_node,
        current_difficulty,
        cumulative_difficulty,
        miner_hashrate: format_hashrate(miner_hashrate),
        network_hashrate: format_hashrate(network_hashrate),
        current_round,
        genesis,
        recent_blocks,
        first_blocks,
        finalized_rounds,
        epoch_nominees,
        sequencer_committee,
        sequencer_nomination_epoch,
        named_validators,
        validator_min_stake,
        validator_qc_numerator,
        validator_qc_denominator,
        dest_apply_requires_cert,
        recent_prefix_certs,
        pending_prefix_cert_requests,
    })
}

fn block_status(
    block: &MinerBlock,
    block_map: &std::collections::HashMap<u64, &MinerBlock>,
) -> BlockStatus {
    let time_delta = if block.index == 0 {
        "-".to_string()
    } else if let Some(parent) = block_map.get(&(block.index - 1)) {
        (block.timestamp - parent.timestamp).to_string()
    } else {
        "N/A".to_string()
    };
    BlockStatus {
        index: block.index,
        epoch: block.epoch,
        hash: block.hash.clone(),
        nominee: block.nominated_peer_id.clone(),
        timestamp: block.timestamp,
        time_delta,
    }
}

fn prefix_cert_status(v: &serde_json::Value) -> PrefixCertStatus {
    PrefixCertStatus {
        source_contract: v
            .get("source_contract")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string(),
        through_commit: v
            .get("through_commit")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string(),
        validator_peer_id: v
            .get("validator_peer_id")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string(),
        prefix_digest: v
            .get("prefix_digest")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string(),
    }
}

async fn collect_finalized_rounds(mgr: &DatastoreManager, current_round: u64) -> Vec<RoundStatus> {
    let mut rounds = Vec::new();
    let start_round = current_round.saturating_sub(STATUS_FINALIZED_ROUNDS_TO_SHOW);
    for round_id in (start_round..current_round).rev() {
        let all_blocks = match ValidatorBlock::find_all_in_round_multi(mgr, round_id).await {
            Ok(blocks) => blocks,
            Err(_) => continue,
        };
        if all_blocks.is_empty() {
            continue;
        }
        let certified_count = all_blocks.iter().filter(|b| b.cert.is_some()).count();
        let completion_pct = if all_blocks.is_empty() {
            0.0
        } else {
            (certified_count as f32 / all_blocks.len() as f32) * 100.0
        };
        let status = if completion_pct >= BFT_THRESHOLD_PERCENTAGE {
            "Finalized"
        } else if completion_pct > 0.0 {
            "Partial"
        } else {
            "In Progress"
        };
        rounds.push(RoundStatus {
            round_id,
            certified_count,
            total_count: all_blocks.len(),
            status,
        });
    }
    rounds
}

fn collect_epoch_nominees(
    miner_blocks: &[MinerBlock],
    current_epoch: u64,
    blocks_per_epoch: u64,
) -> Vec<EpochNominees> {
    let mut out = Vec::new();
    if current_epoch == 0 || blocks_per_epoch == 0 {
        return out;
    }
    let epochs_to_show = std::cmp::min(STATUS_EPOCHS_TO_SHOW, current_epoch);
    for epoch_offset in 1..=epochs_to_show {
        let epoch = current_epoch - epoch_offset;
        let epoch_start = epoch * blocks_per_epoch;
        let epoch_end = epoch_start + blocks_per_epoch;
        let epoch_blocks: Vec<&MinerBlock> = miner_blocks
            .iter()
            .filter(|b| b.index >= epoch_start && b.index < epoch_end)
            .collect();
        if epoch_blocks.len() != blocks_per_epoch as usize {
            continue;
        }
        let mut seed = 0u64;
        for block in &epoch_blocks {
            if let Ok(nonce) = block.nonce.parse::<u128>() {
                seed ^= nonce as u64;
            }
        }
        let shuffled_indices =
            modality_common::shuffle::fisher_yates_shuffle(seed, epoch_blocks.len());
        let nominees: Vec<EpochNominee> = shuffled_indices
            .into_iter()
            .enumerate()
            .map(|(rank, original_idx)| {
                let block = epoch_blocks[original_idx];
                EpochNominee {
                    rank: rank + 1,
                    block_index: block.index,
                    block_hash: block.hash.clone(),
                    peer_id: block.nominated_peer_id.clone(),
                }
            })
            .collect();
        out.push(EpochNominees { epoch, nominees });
    }
    out
}

fn calculate_network_hashrate(miner_blocks: &[MinerBlock]) -> f64 {
    if miner_blocks.len() < 2 {
        return 0.0;
    }
    let recent_count = std::cmp::min(NETWORK_HASHRATE_SAMPLE_SIZE, miner_blocks.len());
    let recent_blocks: Vec<_> = {
        let mut sorted = miner_blocks.to_vec();
        sorted.sort_by_key(|b| b.index);
        sorted.into_iter().rev().take(recent_count).collect()
    };
    if recent_blocks.len() < 2 {
        return 0.0;
    }
    let oldest_block = recent_blocks.last().unwrap();
    let newest_block = recent_blocks.first().unwrap();
    let time_span = (newest_block.timestamp - oldest_block.timestamp) as f64;
    let num_intervals = (newest_block.index - oldest_block.index) as f64;
    if time_span <= 0.0 || num_intervals <= 0.0 {
        return 0.0;
    }
    let avg_block_time = time_span / num_intervals;
    let total_difficulty: u128 = recent_blocks
        .iter()
        .filter_map(|b| b.target_difficulty.parse::<u128>().ok())
        .sum();
    let avg_difficulty = total_difficulty as f64 / recent_blocks.len() as f64;
    const DIFFICULTY_SCALE_FACTOR: f64 = 256.0;
    if avg_block_time > 0.0 {
        (avg_difficulty * DIFFICULTY_SCALE_FACTOR) / avg_block_time
    } else {
        0.0
    }
}

/// Format hashrate for display (with K, M, G, T suffixes)
pub fn format_hashrate(hashrate: f64) -> String {
    if hashrate == 0.0 {
        return "0".to_string();
    }
    if hashrate < 1_000.0 {
        format!("{:.2}", hashrate)
    } else if hashrate < 1_000_000.0 {
        format!("{:.2} K", hashrate / 1_000.0)
    } else if hashrate < 1_000_000_000.0 {
        format!("{:.2} M", hashrate / 1_000_000.0)
    } else if hashrate < 1_000_000_000_000.0 {
        format!("{:.2} G", hashrate / 1_000_000_000.0)
    } else {
        format!("{:.2} T", hashrate / 1_000_000_000_000.0)
    }
}

#[cfg(test)]
pub(crate) fn sample_status() -> NodeStatus {
    NodeStatus {
        peerid: String::new(),
        role: String::new(),
        role_display: "Miner".into(),
        active_roles: vec!["Miner"],
        network_name: String::new(),
        hybrid_consensus: true,
        listeners: vec![],
        status_url: None,
        connected_peers: 0,
        peers: vec![],
        chain_tip: 90,
        current_epoch: 2,
        blocks_per_epoch: 40,
        total_miner_blocks: 91,
        blocks_mined_by_node: 0,
        current_difficulty: "1".into(),
        cumulative_difficulty: 1,
        miner_hashrate: "0".into(),
        network_hashrate: "0".into(),
        current_round: 0,
        genesis: None,
        recent_blocks: vec![],
        first_blocks: vec![],
        finalized_rounds: vec![],
        epoch_nominees: vec![],
        sequencer_committee: vec![],
        sequencer_nomination_epoch: None,
        named_validators: vec![],
        validator_min_stake: 0,
        validator_qc_numerator: 2,
        validator_qc_denominator: 3,
        dest_apply_requires_cert: false,
        recent_prefix_certs: vec![],
        pending_prefix_cert_requests: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_node_role_maps_protocol_names() {
        assert_eq!(display_node_role("miner"), "Miner");
        assert_eq!(display_node_role("hybrid"), "Miner+Sequencer");
        assert_eq!(display_node_role("Miner+Validator"), "Miner+Sequencer");
        assert_eq!(display_node_role("validator"), "Sequencer");
        assert_eq!(display_node_role("sequencer"), "Sequencer");
        assert_eq!(display_node_role("contract-validator"), "Validator");
        assert_eq!(display_node_role("observer"), "Observer");
    }

    #[test]
    fn enumerated_roles_join_with_plus() {
        assert_eq!(
            format_enumerated_roles(&["Miner", "Sequencer", "Validator"]),
            "Miner+Sequencer+Validator"
        );
    }

    #[test]
    fn format_hashrate_uses_suffixes() {
        assert_eq!(format_hashrate(0.0), "0");
        assert_eq!(format_hashrate(12.5), "12.50");
        assert_eq!(format_hashrate(2_500.0), "2.50 K");
        assert_eq!(format_hashrate(3_000_000.0), "3.00 M");
    }

    #[test]
    fn epoch_progress_is_fraction_of_blocks_per_epoch() {
        let status = sample_status();
        assert!((status.epoch_progress() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn hybrid_miner_shows_miner_and_sequencer() {
        let roles = derive_active_roles("hybrid", true, false, false, true, &[], "peer");
        assert_eq!(roles, vec!["Miner", "Sequencer"]);
        assert_eq!(format_enumerated_roles(&roles), "Miner+Sequencer");
    }

    #[test]
    fn hybrid_named_validator_enumerates_all_three() {
        let named = vec!["peer".to_string()];
        let roles = derive_active_roles("hybrid", true, false, false, true, &named, "peer");
        assert_eq!(roles, vec!["Miner", "Sequencer", "Validator"]);
        assert_eq!(format_enumerated_roles(&roles), "Miner+Sequencer+Validator");
    }

    #[test]
    fn named_peer_is_validator() {
        let named = vec!["peer1".to_string()];
        let roles = derive_active_roles("miner", true, false, false, true, &named, "peer1");
        assert!(roles.contains(&"Validator"));
    }

    #[test]
    fn observer_is_none_of_the_three() {
        let roles = derive_active_roles("observer", false, false, false, true, &[], "peer");
        assert!(roles.is_empty());
    }

    #[test]
    fn contract_validator_role_is_validator_only() {
        let roles =
            derive_active_roles("contract-validator", false, false, true, false, &[], "peer");
        assert_eq!(roles, vec!["Validator"]);
    }
}
