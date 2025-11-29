//! Block production and gossip.
//!
//! This module contains the `mine_and_gossip_block` function that handles
//! mining a single block and announcing it to peers.

use anyhow::Result;
use libp2p::gossipsub::IdentTopic;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::gossip;
use crate::constants::{BLOCKS_PER_EPOCH, ROLLING_INTEGRITY_CHECK_INTERVAL, ROLLING_INTEGRITY_WINDOW};
use super::mining_loop::MiningOutcome;

/// Mine a block and gossip it to peers.
pub async fn mine_and_gossip_block(
    index: u64,
    peer_id: &str,
    miner_nominees: &Option<Vec<String>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    fork_config: modal_observer::ForkConfig,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    initial_difficulty: Option<u128>,
    miner_hash_func: Option<String>,
    miner_hash_params: Option<serde_json::Value>,
    mining_delay_ms: Option<u64>,
    epoch_transition_tx: Option<tokio::sync::broadcast::Sender<u64>>,
) -> Result<MiningOutcome> {
    use modal_miner::{Blockchain, ChainConfig};
    
    // Determine the nominee
    let nominated_peer_id = match miner_nominees {
        Some(nominees) if !nominees.is_empty() => {
            let nominee_index = (index as usize) % nominees.len();
            nominees[nominee_index].clone()
        }
        _ => peer_id.to_string(),
    };

    log::info!("Mining block {} with nominated peer: {}", index, nominated_peer_id);

    // Create ChainConfig
    let chain_config = ChainConfig {
        initial_difficulty: initial_difficulty.unwrap_or(1000),
        target_block_time_secs: 60,
        mining_delay_ms,
    };

    // Load blockchain
    let mut chain = Blockchain::load_or_create_with_fork_config(
        chain_config,
        peer_id.to_string(),
        datastore.clone(),
        fork_config,
    ).await?;
    
    log::info!("Loaded chain with {} blocks (height: {})", chain.blocks.len(), chain.height());
    
    // Determine hash function
    let (final_hash_func, final_hash_params) = get_hash_config(
        &datastore,
        miner_hash_func,
        miner_hash_params,
    ).await;
    
    // Set RandomX parameters if needed
    if final_hash_func == "randomx" && final_hash_params.is_some() {
        modal_common::hash_tax::set_randomx_params_from_json(final_hash_params.as_ref());
        log::info!("Set custom RandomX parameters for mining");
    }
    
    // Create miner with hash function
    let custom_miner = modal_miner::Miner::new(modal_miner::MinerConfig {
        max_tries: None,
        hash_func_name: Some(final_hash_func.leak()),
        mining_delay_ms: chain.config.mining_delay_ms,
    });
    chain.miner = custom_miner;
    
    // Check if block already exists
    if index < chain.height() + 1 && index < chain.blocks.len() as u64 {
        log::warn!("⏭️  Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
        return Ok(MiningOutcome::Skipped);
    }
    
    // Verify correct next block
    let expected_next = chain.height() + 1;
    if index != expected_next {
        log::error!("Index mismatch: expected to mine block {}, but was asked to mine block {}", expected_next, index);
        return Err(anyhow::anyhow!("Index mismatch: chain expects block {} but trying to mine {}", expected_next, index));
    }
    
    log::info!("Chain ready for mining. Height: {}, Mining next index: {}", chain.height(), index);
    
    // Mine the block
    let miner_number = rand::random::<u64>();
    let (mined_block, mining_stats) = chain.mine_block_with_persistence(
        nominated_peer_id.clone(),
        miner_number
    ).await?;
    
    // Update metrics
    if let Some(stats) = mining_stats {
        let mut metrics = mining_metrics.write().await;
        metrics.record_block_mined(stats.attempts as u64, stats.duration_secs);
        
        log::info!("⛏️  Block {} mined: {} attempts in {:.2}s, instant: {:.2} H/s",
            index, stats.attempts, stats.duration_secs, stats.hashrate());
        log::info!("📊 Miner Stats: avg_hashrate={:.2} H/s, total_blocks={}, total_hashes={}",
            metrics.average_hashrate(), metrics.blocks_mined, metrics.total_hashes);
    }
    
    // Verify mined block index
    if mined_block.header.index != index {
        log::error!("Mined block index mismatch: expected {}, got {}", index, mined_block.header.index);
        return Err(anyhow::anyhow!("Mined block index mismatch"));
    }

    // Convert to MinerBlock
    let miner_block = MinerBlock::new_canonical(
        mined_block.header.hash.clone(),
        index,
        index / BLOCKS_PER_EPOCH,
        mined_block.header.timestamp.timestamp(),
        mined_block.header.previous_hash.clone(),
        mined_block.header.data_hash.clone(),
        mined_block.header.nonce,
        mined_block.header.difficulty,
        mined_block.data.nominated_peer_id.clone(),
        mined_block.data.miner_number,
    );

    // Gossip the block
    gossip_block(&swarm, &miner_block).await;

    log::info!("Mined block {} (epoch {}) with hash {} and difficulty {}",
        miner_block.index, miner_block.epoch, &miner_block.hash[..16], miner_block.difficulty);
    
    // Rolling integrity check
    if miner_block.index > 0 && miner_block.index % ROLLING_INTEGRITY_CHECK_INTERVAL == 0 {
        run_integrity_check(&datastore, miner_block.index).await;
    }
    
    // Log epoch changes
    if miner_block.index > 0 && miner_block.index % BLOCKS_PER_EPOCH == 0 {
        log::info!("🎯 EPOCH {} STARTED - New difficulty: {}", miner_block.epoch, miner_block.difficulty);
        
        if let Some(tx) = epoch_transition_tx {
            if let Err(e) = tx.send(miner_block.epoch) {
                log::debug!("No receivers for epoch transition: {}", e);
            } else {
                log::info!("📡 Broadcasted epoch {} transition for validator coordination", miner_block.epoch);
            }
        }
    }

    Ok(MiningOutcome::Mined)
}

/// Get hash configuration from genesis contract or config
async fn get_hash_config(
    datastore: &Arc<Mutex<DatastoreManager>>,
    miner_hash_func: Option<String>,
    miner_hash_params: Option<serde_json::Value>,
) -> (String, Option<serde_json::Value>) {
    let datastore_guard = datastore.lock().await;
    let genesis_params = datastore_guard.get_string("/network/genesis_contract_id").await.ok()
        .flatten()
        .and_then(|contract_id| {
            futures::executor::block_on(async {
                datastore_guard.load_network_parameters_from_contract(&contract_id).await.ok()
            })
        });
    drop(datastore_guard);
    
    if let Some(params) = genesis_params {
        log::info!("Using miner hash configuration from genesis contract: {}", params.miner_hash_func);
        (params.miner_hash_func, params.mining_hash_params)
    } else {
        let hash_func = miner_hash_func.unwrap_or_else(|| {
            log::info!("Using default miner hash function: randomx");
            "randomx".to_string()
        });
        log::info!("Using miner hash configuration from node config: {}", hash_func);
        (hash_func, miner_hash_params)
    }
}

/// Gossip a block to peers
async fn gossip_block(swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>, miner_block: &MinerBlock) {
    let gossip_msg = gossip::miner::block::MinerBlockGossip::from_miner_block(miner_block);
    let topic = IdentTopic::new(gossip::miner::block::TOPIC);
    
    if let Ok(json) = serde_json::to_string(&gossip_msg) {
        let mut swarm_lock = swarm.lock().await;
        match swarm_lock.behaviour_mut().gossipsub.publish(topic, json.as_bytes()) {
            Ok(_) => {
                log::debug!("Gossipped block {} to peers", miner_block.index);
            }
            Err(e) => {
                log::debug!("Could not gossip block {} (no peers available): {}", miner_block.index, e);
            }
        }
    }
}

/// Run rolling integrity check
async fn run_integrity_check(datastore: &Arc<Mutex<DatastoreManager>>, current_index: u64) {
    let ds = datastore.lock().await;
    match crate::actions::chain_integrity::check_recent_blocks(&ds, ROLLING_INTEGRITY_WINDOW, true).await {
        Ok(true) => {
            log::debug!("✓ Rolling integrity check passed (last {} blocks)", ROLLING_INTEGRITY_WINDOW);
        }
        Ok(false) => {
            log::error!("❌ Rolling integrity check found and repaired broken blocks");
        }
        Err(e) => {
            log::error!("⚠️  Rolling integrity check failed: {}", e);
        }
    }
}

