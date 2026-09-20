//! Helper functions for the Node module.
//!
//! This module contains helper functions that support Node operations
//! but don't need to be part of the Node impl block.

use anyhow::Result;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;

use crate::config::Config;
use crate::inspection::{
    DatastoreInfo, InspectionData, InspectionLevel, MiningInfo, NetworkInfo, NodeStatus,
};

/// Extract PeerId from a Multiaddr
pub fn extract_peer_id(multiaddr: Multiaddr) -> Option<PeerId> {
    let protocols: Vec<Protocol> = multiaddr.iter().collect();
    let last_protocol = protocols.last()?;

    match last_protocol {
        Protocol::P2p(peer_id) => Some(*peer_id),
        _ => None,
    }
}

/// Exclude multiaddresses that contain a specific PeerId
pub fn exclude_multiaddresses_with_peerid(ma: Vec<Multiaddr>, peerid: PeerId) -> Vec<Multiaddr> {
    ma.into_iter()
        .filter(|addr| {
            if let Some(Protocol::P2p(addr_peerid)) = addr.iter().last() {
                addr_peerid != peerid
            } else {
                true
            }
        })
        .collect()
}

/// Initialize the DatastoreManager from config
pub async fn initialize_datastore(config: &Config) -> Result<Arc<Mutex<DatastoreManager>>> {
    let datastore_manager = if let Some(data_dir) = config.data_dir.clone() {
        log::info!("📁 Initializing DatastoreManager at {:?}", data_dir);
        let mgr = DatastoreManager::open(&data_dir)?;
        log::info!("✓ DatastoreManager initialized with 6 stores");
        Arc::new(Mutex::new(mgr))
    } else if let Some(storage_path) = config.storage_path.clone() {
        log::info!("📁 Using storage_path as data_dir: {:?}", storage_path);
        let mgr = DatastoreManager::open(&storage_path)?;
        log::info!("✓ DatastoreManager initialized with 6 stores");
        Arc::new(Mutex::new(mgr))
    } else {
        log::info!("📁 Creating in-memory DatastoreManager");
        let mgr = DatastoreManager::create_in_memory()?;
        log::info!("✓ In-memory DatastoreManager initialized");
        Arc::new(Mutex::new(mgr))
    };

    Ok(datastore_manager)
}

/// Load network configuration into the datastore
pub async fn load_network_config(
    datastore_manager: &Arc<Mutex<DatastoreManager>>,
    network_config_path: PathBuf,
) -> Result<()> {
    let network_config = if let Some(network_name) = network_config_path
        .to_string_lossy()
        .strip_prefix("modality-networks://")
    {
        log::info!(
            "Loading network config from modality-networks: {}",
            network_name
        );
        let network_info = modality_networks::networks::by_name(network_name).ok_or_else(|| {
            anyhow::anyhow!("Network '{}' not found in modality-networks", network_name)
        })?;

        let mut config_json = serde_json::json!({
            "name": network_info.name,
            "description": network_info.description,
            "bootstrappers": network_info.bootstrappers,
        });

        if let Some(validators) = network_info.validators {
            log::info!(
                "📋 Found {} static validators in network config",
                validators.len()
            );
            config_json["validators"] = serde_json::json!(validators);
        }

        if let Some(contract_validators) = network_info.contract_validators {
            config_json["contract_validators"] = serde_json::json!(contract_validators);
        }
        config_json["validator_min_stake"] = serde_json::json!(network_info.validator_min_stake);
        if let Some(fees) = network_info.validation_fees {
            config_json["validation_fees"] = serde_json::json!({
                "nominal": fees.nominal,
                "meter_coefficient": fees.meter_coefficient,
            });
        }
        config_json["repost_requires_validator_cert"] =
            serde_json::json!(network_info.repost_requires_validator_cert);

        config_json["rounds"] = serde_json::json!({});

        log::debug!(
            "Network config JSON: {}",
            serde_json::to_string_pretty(&config_json).unwrap_or_default()
        );

        config_json
    } else {
        let config_str = std::fs::read_to_string(&network_config_path)?;
        serde_json::from_str(&config_str)?
    };

    // Load network config into NodeState store
    {
        let mgr = datastore_manager.lock().await;
        mgr.load_network_config(&network_config).await?;
    }

    // Load network parameters from genesis contract if present
    if let Some(genesis_contract_id) = network_config
        .get("genesis_contract_id")
        .and_then(|v| v.as_str())
    {
        log::info!(
            "Loading network parameters from genesis contract: {}",
            genesis_contract_id
        );
        let mgr = datastore_manager.lock().await;
        match mgr
            .load_network_parameters_from_contract(genesis_contract_id)
            .await
        {
            Ok(params) => {
                log::info!("✓ Loaded network parameters from contract:");
                log::info!("  Name: {}", params.name);
                log::info!("  Description: {}", params.description);
                log::info!("  Difficulty: {}", params.initial_difficulty);
                log::info!("  Block Time: {}s", params.target_block_time_secs);
                log::info!("  Blocks per Epoch: {}", params.blocks_per_epoch);
                log::info!("  Validators: {}", params.validators.len());

                if !params.validators.is_empty() {
                    mgr.set_static_validators(&params.validators).await?;
                }
            }
            Err(e) => {
                log::warn!("Failed to load network parameters from contract: {}", e);
                log::warn!("Falling back to latest_parameters from config");
            }
        }
    } else {
        log::info!("No genesis_contract_id found in network config");
    }

    Ok(())
}

/// Get inspection data about the node
pub async fn get_inspection_data(
    node: &super::Node,
    level: InspectionLevel,
) -> Result<InspectionData> {
    let peer_id = node.peerid.to_string();
    let mut data = InspectionData::new_basic(peer_id, NodeStatus::Running);

    // Network information
    if InspectionData::should_include_network(level) {
        let swarm = node.swarm.lock().await;
        let connected_peers: Vec<_> = swarm.connected_peers().cloned().collect();
        let connected_peer_list = if InspectionData::should_include_detailed_peers(level) {
            Some(connected_peers.iter().map(|p| p.to_string()).collect())
        } else {
            None
        };

        data.network = Some(NetworkInfo {
            listeners: node.listeners.iter().map(|a| a.to_string()).collect(),
            connected_peers: connected_peers.len(),
            connected_peer_list,
            bootstrappers: node.bootstrappers.iter().map(|a| a.to_string()).collect(),
        });
    }

    // Datastore information
    if InspectionData::should_include_datastore(level) {
        let mgr = node.datastore_manager.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&mgr).await?;

        let block_range = if !blocks.is_empty() {
            Some((blocks.first().unwrap().index, blocks.last().unwrap().index))
        } else {
            None
        };

        let chain_tip_height = blocks.last().map(|b| b.index);
        let chain_tip_hash = blocks.last().map(|b| b.hash.clone());

        let mut epochs_set = std::collections::HashSet::new();
        for block in &blocks {
            epochs_set.insert(block.epoch);
        }

        let mut miners_set = std::collections::HashSet::new();
        for block in &blocks {
            miners_set.insert(&block.nominated_peer_id);
        }

        data.datastore = Some(DatastoreInfo {
            total_blocks: blocks.len(),
            block_range,
            chain_tip_height,
            chain_tip_hash,
            epochs: Some(epochs_set.len()),
            unique_miners: Some(miners_set.len()),
        });
    }

    // Mining information
    if InspectionData::should_include_mining(level) {
        let is_mining = node.mining_shutdown.is_some();
        let nominees = node.miner_nominees.clone();

        let metrics = node.mining_metrics.read().await;
        let (current_hashrate, total_hashes) = if is_mining {
            (Some(metrics.current_hashrate), Some(metrics.total_hashes))
        } else {
            (None, None)
        };

        data.mining = Some(MiningInfo {
            is_mining,
            nominees,
            current_hashrate,
            total_hashes,
        });
    }

    Ok(data)
}
