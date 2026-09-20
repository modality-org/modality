//! Structured node status for the HTTP page and the terminal UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use libp2p::Multiaddr;
use modality_datastore::models::miner::MinerBlock;
use modality_datastore::models::validator::ValidatorBlock;
use modality_datastore::DatastoreManager;

use crate::constants::{
    BFT_THRESHOLD_PERCENTAGE, NETWORK_HASHRATE_SAMPLE_SIZE, STATUS_EPOCHS_TO_SHOW,
    STATUS_FINALIZED_ROUNDS_TO_SHOW, STATUS_RECENT_BLOCKS_COUNT,
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
    pub miner_hashrate: String,
    pub network_hashrate: String,
    pub current_round: u64,
    pub recent_blocks: Vec<BlockStatus>,
    pub finalized_rounds: Vec<RoundStatus>,
    pub epoch_nominees: Vec<EpochNominees>,
}

#[derive(Debug, Clone)]
pub struct PeerStatus {
    pub peer_id: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlockStatus {
    pub index: u64,
    pub epoch: u64,
    pub hash: String,
    pub nominee: String,
}

#[derive(Debug, Clone)]
pub struct RoundStatus {
    pub round_id: u64,
    pub certified_count: usize,
    pub total_count: usize,
    pub status: &'static str,
}

#[derive(Debug, Clone)]
pub struct EpochNominees {
    pub epoch: u64,
    pub nominees: Vec<String>,
}

impl NodeStatus {
    pub fn epoch_progress(&self) -> f64 {
        if self.blocks_per_epoch == 0 {
            return 0.0;
        }
        (self.chain_tip % self.blocks_per_epoch) as f64 / self.blocks_per_epoch as f64
    }
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

    let network_hashrate = calculate_network_hashrate(&miner_blocks);
    let miner_hashrate = {
        let metrics = source.mining_metrics.read().await;
        metrics.average_hashrate()
    };

    let mut recent_blocks: Vec<BlockStatus> = miner_blocks
        .iter()
        .map(|b| BlockStatus {
            index: b.index,
            epoch: b.epoch,
            hash: b.hash.clone(),
            nominee: b.nominated_peer_id.clone(),
        })
        .collect();
    recent_blocks.sort_by_key(|b| std::cmp::Reverse(b.index));
    recent_blocks.truncate(STATUS_RECENT_BLOCKS_COUNT.min(16));

    let mut peers = Vec::new();
    for peer_id in &peer_ids {
        let peer_id_str = peer_id.to_string();
        let role = modality_datastore::models::PeerInfo::find_one(&mgr, &peer_id_str)
            .await
            .ok()
            .flatten()
            .and_then(|info| info.role);
        peers.push(PeerStatus {
            peer_id: peer_id_str,
            role,
        });
    }

    let finalized_rounds = collect_finalized_rounds(&mgr, current_round).await;
    let epoch_nominees = collect_epoch_nominees(&miner_blocks, current_epoch, blocks_per_epoch);
    drop(mgr);

    Ok(NodeStatus {
        peerid: peerid_str,
        role: source.role.clone(),
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
        miner_hashrate: format_hashrate(miner_hashrate),
        network_hashrate: format_hashrate(network_hashrate),
        current_round,
        recent_blocks,
        finalized_rounds,
        epoch_nominees,
    })
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
            "In progress"
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
        let mut nominees: Vec<String> = shuffled_indices
            .into_iter()
            .map(|original_idx| epoch_blocks[original_idx].nominated_peer_id.clone())
            .collect();
        nominees.truncate(8);
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
mod tests {
    use super::*;

    #[test]
    fn format_hashrate_uses_suffixes() {
        assert_eq!(format_hashrate(0.0), "0");
        assert_eq!(format_hashrate(12.5), "12.50");
        assert_eq!(format_hashrate(2_500.0), "2.50 K");
        assert_eq!(format_hashrate(3_000_000.0), "3.00 M");
    }

    #[test]
    fn epoch_progress_is_fraction_of_blocks_per_epoch() {
        let status = NodeStatus {
            peerid: String::new(),
            role: String::new(),
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
            miner_hashrate: "0".into(),
            network_hashrate: "0".into(),
            current_round: 0,
            recent_blocks: vec![],
            finalized_rounds: vec![],
            epoch_nominees: vec![],
        };
        assert!((status.epoch_progress() - 0.25).abs() < f64::EPSILON);
    }
}
