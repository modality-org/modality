//! HTTP status server for node monitoring.
//!
//! This module provides a web-based status page for monitoring node health,
//! blockchain state, and mining statistics.

use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex;
use warp::Filter;

use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;

use crate::constants::{
    BLOCKS_PER_EPOCH, STATUS_PAGE_REFRESH_SECS, STATUS_RECENT_BLOCKS_COUNT,
    STATUS_FIRST_BLOCKS_COUNT, STATUS_EPOCHS_TO_SHOW, NETWORK_HASHRATE_SAMPLE_SIZE,
};
use crate::templates::{
    render_block_row, render_peer_row, render_listener_item,
    render_block_0_info, render_block_0_not_found, render_empty_blocks_message,
    render_empty_peers_message, render_epoch_nominees_section, render_nominee_row,
    render_status_page, StatusPageVars,
};

/// Start HTTP status server on the specified port
pub async fn start_status_server(
    port: u16,
    peerid: libp2p_identity::PeerId,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    network_name: String,
    role: String,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    let status_route = warp::path::end()
        .and(warp::get())
        .and(with_peerid(peerid))
        .and(with_datastore(datastore_manager.clone()))
        .and(with_swarm(swarm.clone()))
        .and(with_listeners(listeners.clone()))
        .and(with_mining_metrics(mining_metrics.clone()))
        .and(with_network_name(network_name.clone()))
        .and(with_role(role.clone()))
        .and_then(status_handler);

    let routes = status_route;

    log::info!("Starting HTTP status server on http://0.0.0.0:{}", port);

    let server = warp::serve(routes).bind(([0, 0, 0, 0], port));

    let handle = tokio::spawn(async move {
        server.await;
    });

    Ok(handle)
}

fn with_mining_metrics(
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
) -> impl Filter<Extract = (crate::mining_metrics::SharedMiningMetrics,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mining_metrics.clone())
}

fn with_network_name(
    network_name: String,
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || network_name.clone())
}

fn with_role(
    role: String,
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || role.clone())
}

fn with_peerid(
    peerid: libp2p_identity::PeerId,
) -> impl Filter<Extract = (libp2p_identity::PeerId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peerid)
}

fn with_datastore(
    datastore_manager: Arc<Mutex<DatastoreManager>>,
) -> impl Filter<Extract = (Arc<Mutex<DatastoreManager>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || datastore_manager.clone())
}

fn with_swarm(
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
) -> impl Filter<Extract = (Arc<Mutex<crate::swarm::NodeSwarm>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || swarm.clone())
}

fn with_listeners(
    listeners: Vec<libp2p::Multiaddr>,
) -> impl Filter<Extract = (Vec<libp2p::Multiaddr>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || listeners.clone())
}

/// Generate status HTML content
pub async fn generate_status_html(
    peerid: libp2p_identity::PeerId,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    network_name: String,
    role: String,
) -> Result<String, anyhow::Error> {
    // Get connected peers information
    let peer_info = {
        let swarm_lock = swarm.lock().await;
        swarm_lock.connected_peers().cloned().collect::<Vec<_>>()
    };
    let connected_peers = peer_info.len();
    
    // Get node status information
    let mgr = datastore_manager.lock().await;
    let current_round = mgr.get_current_round().await.unwrap_or(0);
    let latest_round = 0; // TODO: Implement find_max_int_key in DatastoreManager
    
    // Get miner blocks information
    let miner_blocks = MinerBlock::find_all_canonical_multi(&mgr).await.unwrap_or_default();
    let total_miner_blocks = miner_blocks.len();
    
    // Get latest block for current difficulty
    let latest_block = miner_blocks.iter().max_by_key(|b| b.index);
    let current_difficulty = latest_block
        .map(|b| b.difficulty.clone())
        .unwrap_or_else(|| "0".to_string());
    let current_epoch = latest_block.map(|b| b.epoch).unwrap_or(0);
    
    // Calculate cumulative difficulty
    let cumulative_difficulty: u128 = miner_blocks
        .iter()
        .filter_map(|block| block.difficulty.parse::<u128>().ok())
        .sum();
    
    // Count blocks mined by this node
    let peerid_str = peerid.to_string();
    let blocks_mined_by_node = miner_blocks
        .iter()
        .filter(|block| block.nominated_peer_id == peerid_str)
        .count();
    
    // Calculate network hashrate from recent blocks
    let network_hashrate = calculate_network_hashrate(&miner_blocks);
    
    // Get miner hashrate (average over all mining activity)
    let miner_hashrate = {
        let metrics = mining_metrics.read().await;
        metrics.average_hashrate()
    };
    
    // Get Block 0 (genesis block)
    let block_0 = MinerBlock::find_canonical_by_index_simple(&mgr, 0).await.ok().flatten();
    
    // Get last N blocks (sorted by index descending)
    let mut recent_blocks = miner_blocks.clone();
    recent_blocks.sort_by(|a, b| b.index.cmp(&a.index));
    recent_blocks.truncate(STATUS_RECENT_BLOCKS_COUNT);
    
    // Get first N blocks (sorted by index ascending)
    let mut first_blocks = miner_blocks.clone();
    first_blocks.sort_by(|a, b| a.index.cmp(&b.index));
    first_blocks.truncate(STATUS_FIRST_BLOCKS_COUNT);
    
    // Create a map of block index to block for quick parent lookup
    let block_map: std::collections::HashMap<u64, &MinerBlock> = miner_blocks
        .iter()
        .map(|block| (block.index, block))
        .collect();
    
    // Calculate epoch nominees with shuffle order for previous epochs
    let epoch_nominees_data = calculate_epoch_nominees(&miner_blocks, current_epoch);
    
    drop(mgr);

    // Build blocks table HTML for recent blocks
    let blocks_html = build_blocks_html(&recent_blocks, &block_map);
    let first_blocks_html = build_blocks_html(&first_blocks, &block_map);
    
    // Build Block 0 HTML
    let block_0_html = match block_0 {
        Some(ref block) => render_block_0_info(
            block.index,
            &block.hash,
            block.epoch,
            block.timestamp,
            &block.previous_hash,
            &block.data_hash,
            &block.difficulty,
            &block.nominated_peer_id,
        ),
        None => render_block_0_not_found(),
    };

    // Build peers list HTML
    let peers_html = if peer_info.is_empty() {
        render_empty_peers_message()
    } else {
        peer_info
            .iter()
            .map(|peer_id| render_peer_row(&peer_id.to_string()))
            .collect::<Vec<_>>()
            .join("\n                    ")
    };

    // Build epoch nominees HTML sections
    let epoch_nominees_sections = build_epoch_nominees_html(&epoch_nominees_data);

    // Build listeners HTML
    let listeners_html = listeners
        .iter()
        .map(|l| render_listener_item(&l.to_string()))
        .collect::<Vec<_>>()
        .join("\n                    ");

    // Render the template
    let vars = StatusPageVars {
        refresh_interval: STATUS_PAGE_REFRESH_SECS,
        connected_peers,
        total_miner_blocks,
        cumulative_difficulty,
        peerid: peerid.to_string(),
        network_name,
        role,
        listeners_html,
        current_round,
        latest_round,
        block_0_html,
        peers_html,
        blocks_mined_by_node,
        current_difficulty,
        miner_hashrate: format_hashrate(miner_hashrate),
        network_hashrate: format_hashrate(network_hashrate),
        recent_blocks_count: STATUS_RECENT_BLOCKS_COUNT,
        blocks_html,
        first_blocks_count: STATUS_FIRST_BLOCKS_COUNT,
        first_blocks_html,
        current_epoch,
        completed_epochs: current_epoch,
        epoch_nominees_sections,
    };

    Ok(render_status_page(vars))
}

/// Build HTML for a list of blocks
fn build_blocks_html(
    blocks: &[MinerBlock],
    block_map: &std::collections::HashMap<u64, &MinerBlock>,
) -> String {
    if blocks.is_empty() {
        return render_empty_blocks_message();
    }
    
    blocks
        .iter()
        .map(|block| {
            let time_delta = if block.index == 0 {
                "-".to_string()
            } else if let Some(parent) = block_map.get(&(block.index - 1)) {
                (block.timestamp - parent.timestamp).to_string()
            } else {
                "N/A".to_string()
            };
            
            render_block_row(
                block.index,
                block.epoch,
                &block.hash,
                &block.nominated_peer_id,
                block.timestamp,
                &time_delta,
            )
        })
        .collect::<Vec<_>>()
        .join("\n                    ")
}

/// Calculate epoch nominees data
fn calculate_epoch_nominees(
    miner_blocks: &[MinerBlock],
    current_epoch: u64,
) -> Vec<(u64, Vec<(usize, String, String, u64)>)> {
    let mut epoch_nominees_data = Vec::new();
    
    if current_epoch == 0 {
        return epoch_nominees_data;
    }
    
    let epochs_to_show = std::cmp::min(STATUS_EPOCHS_TO_SHOW, current_epoch);
    
        for epoch_offset in 1..=epochs_to_show {
            let epoch = current_epoch - epoch_offset;
            let epoch_start = epoch * BLOCKS_PER_EPOCH;
            let epoch_end = epoch_start + BLOCKS_PER_EPOCH;
            
            // Get all blocks from this epoch
            let epoch_blocks: Vec<&MinerBlock> = miner_blocks
                .iter()
                .filter(|b| b.index >= epoch_start && b.index < epoch_end)
                .collect();
            
            // Only process complete epochs
            if epoch_blocks.len() == BLOCKS_PER_EPOCH as usize {
                // Calculate XOR seed from all nonces
                let mut seed = 0u64;
                for block in &epoch_blocks {
                    if let Ok(nonce) = block.nonce.parse::<u128>() {
                        seed ^= nonce as u64;
                    }
                }
                
                // Get shuffled indices using Fisher-Yates
                let shuffled_indices = modal_common::shuffle::fisher_yates_shuffle(seed, epoch_blocks.len());
                
                // Map shuffled indices to (shuffle_rank, block_hash, nominated_peer_id, block_index)
                let shuffled_nominees: Vec<(usize, String, String, u64)> = shuffled_indices
                    .into_iter()
                    .enumerate()
                    .map(|(rank, original_idx)| {
                        let block = epoch_blocks[original_idx];
                        (rank, block.hash.clone(), block.nominated_peer_id.clone(), block.index)
                    })
                    .collect();
                
                epoch_nominees_data.push((epoch, shuffled_nominees));
            }
        }
    
    epoch_nominees_data
}

/// Build HTML for epoch nominees sections
fn build_epoch_nominees_html(
    epoch_nominees_data: &[(u64, Vec<(usize, String, String, u64)>)],
) -> String {
    if epoch_nominees_data.is_empty() {
        return String::new();
    }
    
        epoch_nominees_data
            .iter()
            .map(|(epoch, nominees)| {
                let nominees_html = nominees
                    .iter()
                    .map(|(rank, block_hash, peer_id, block_idx)| {
                    render_nominee_row(*rank + 1, *block_idx, block_hash, peer_id)
                    })
                    .collect::<Vec<_>>()
                    .join("\n                    ");

            render_epoch_nominees_section(*epoch, &nominees_html)
            })
            .collect::<Vec<_>>()
            .join("\n")
}

async fn status_handler(
    peerid: libp2p_identity::PeerId,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    network_name: String,
    role: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    let html = generate_status_html(peerid, datastore_manager, swarm, listeners, mining_metrics, network_name, role)
        .await
        .map_err(|_| warp::reject::not_found())?;
    Ok(warp::reply::html(html))
}

/// Calculate network hashrate based on recent blocks
/// Uses the difficulty and block times to estimate the network's total mining power
fn calculate_network_hashrate(miner_blocks: &[MinerBlock]) -> f64 {
    if miner_blocks.len() < 2 {
        return 0.0;
    }
    
    // Use last N blocks for more stable estimate
    let recent_count = std::cmp::min(NETWORK_HASHRATE_SAMPLE_SIZE, miner_blocks.len());
    let recent_blocks: Vec<_> = {
        let mut sorted = miner_blocks.to_vec();
        sorted.sort_by_key(|b| b.index);
        sorted.into_iter().rev().take(recent_count).collect()
    };
    
    if recent_blocks.len() < 2 {
        return 0.0;
    }
    
    // Calculate average time between blocks
    let oldest_block = recent_blocks.last().unwrap();
    let newest_block = recent_blocks.first().unwrap();
    let time_span = (newest_block.timestamp - oldest_block.timestamp) as f64;
    let num_intervals = (newest_block.index - oldest_block.index) as f64;
    
    if time_span <= 0.0 || num_intervals <= 0.0 {
        return 0.0;
    }
    
    let avg_block_time = time_span / num_intervals;
    
    // Calculate average difficulty across recent blocks
    let total_difficulty: u128 = recent_blocks
        .iter()
        .filter_map(|b| b.difficulty.parse::<u128>().ok())
        .sum();
    let avg_difficulty = total_difficulty as f64 / recent_blocks.len() as f64;
    
    // The difficulty system uses: target = (0xffff << (0x1d * 8)) / difficulty
    // For 256-bit hashes (RandomX, SHA256): expected_hashes = difficulty × 2^256 / max_target
    // With max_target = 0xffff << 232 ≈ 2^248, this gives: expected_hashes ≈ difficulty × 256
    // This scaling factor is the same for all 256-bit hash algorithms
    const DIFFICULTY_SCALE_FACTOR: f64 = 256.0;
    
    // Network hashrate = expected_hashes_per_block / block_time
    if avg_block_time > 0.0 {
        (avg_difficulty * DIFFICULTY_SCALE_FACTOR) / avg_block_time
    } else {
        0.0
    }
}

/// Format hashrate for display (with K, M, G, T suffixes)
fn format_hashrate(hashrate: f64) -> String {
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

/// Start status HTML writer task that periodically writes HTML to a directory
pub async fn start_status_html_writer(
    dir: PathBuf,
    peerid: libp2p_identity::PeerId,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    network_name: String,
    role: String,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&dir)?;

    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(STATUS_PAGE_REFRESH_SECS));
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Generate and write HTML
                    match generate_status_html(
                        peerid,
                        datastore_manager.clone(),
                        swarm.clone(),
                        listeners.clone(),
                        mining_metrics.clone(),
                        network_name.clone(),
                        role.clone(),
                    ).await {
                        Ok(html) => {
                            let index_path = dir.join("index.html");
                            if let Err(e) = tokio::fs::write(&index_path, html).await {
                                log::error!("Failed to write status HTML to {}: {}", index_path.display(), e);
                            } else {
                                log::debug!("Status HTML written to {}", index_path.display());
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to generate status HTML: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    log::info!("Status HTML writer task shutting down");
                    break;
                }
            }
        }
    });

    Ok(handle)
}
