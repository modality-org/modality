//! Hybrid consensus: PoW miner chain plus miner-nominated sequencers with N−2 lookback.
//!
//! Validators for mining epoch N are selected from nominations in epoch N−2.
//! Epoch is derived from canonical chain height and `epoch_config.blocks_per_epoch`,
//! not a hardcoded constant, so validator-only nodes and non-boundary miners still
//! observe transitions.

use modality_common::keypair::Keypair;
use modality_datastore::models::validator::get_validator_set_for_mining_epoch_hybrid_multi;
use modality_datastore::DatastoreManager;
use modality_networks::CheckpointMode;
use modality_validator_consensus::communication::Message as ConsensusMessage;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::actions::observer::get_chain_tip_index;
use crate::swarm::NodeSwarm;

use super::consensus::{self, SequencingControl};

/// Mining epoch for a canonical height given the configured blocks-per-epoch.
pub fn mining_epoch_for_height(height: u64, blocks_per_epoch: u64) -> u64 {
    if blocks_per_epoch == 0 {
        0
    } else {
        height / blocks_per_epoch
    }
}

/// Poll canonical height and broadcast mining-epoch changes.
pub fn start_epoch_watch_from_chain(
    datastore: Arc<Mutex<DatastoreManager>>,
    epoch_tx: broadcast::Sender<u64>,
) {
    tokio::spawn(async move {
        let mut last_epoch = get_current_epoch(&datastore).await;
        log::info!("Epoch watcher started at mining epoch {}", last_epoch);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let epoch = get_current_epoch(&datastore).await;
            if epoch != last_epoch {
                log::info!("📡 Canonical height crossed into mining epoch {}", epoch);
                if epoch_tx.send(epoch).is_err() {
                    log::debug!("No receivers for epoch transition");
                }
                last_epoch = epoch;
            }
        }
    });
}

/// Start the hybrid consensus monitor.
///
/// This spawns a background task that monitors epoch transitions and starts
/// consensus if this node is selected as a validator.
pub fn start_hybrid_consensus_monitor(
    datastore: Arc<Mutex<DatastoreManager>>,
    node_peer_id: String,
    epoch_rx: broadcast::Receiver<u64>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
) {
    start_hybrid_consensus_monitor_with_checkpoints(
        datastore,
        node_peer_id,
        epoch_rx,
        keypair,
        swarm,
        consensus_tx,
        consensus_rx,
        CheckpointMode::None,
    )
}

/// Start the hybrid consensus monitor with checkpoint support.
pub fn start_hybrid_consensus_monitor_with_checkpoints(
    datastore: Arc<Mutex<DatastoreManager>>,
    node_peer_id: String,
    mut epoch_rx: broadcast::Receiver<u64>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    checkpoint_mode: CheckpointMode,
) {
    let consensus_rx = Arc::new(Mutex::new(Some(consensus_rx)));
    let control = SequencingControl {
        participate: Arc::new(AtomicBool::new(false)),
        committee_size: Arc::new(AtomicUsize::new(0)),
        started: Arc::new(AtomicBool::new(false)),
    };

    tokio::spawn(async move {
        log::info!("Hybrid consensus coordinator started, waiting for epoch >= 2...");
        log::info!("Checkpoint mode: {:?}", checkpoint_mode);

        let current_epoch = get_current_epoch(&datastore).await;

        if current_epoch >= 2 {
            log::info!(
                "Current epoch is {}, checking validator set immediately",
                current_epoch
            );
            check_and_start_validator(
                &datastore,
                &node_peer_id,
                current_epoch,
                &keypair,
                swarm.clone(),
                consensus_tx.clone(),
                consensus_rx.clone(),
                control.clone(),
                checkpoint_mode.clone(),
            )
            .await;
        }

        loop {
            match epoch_rx.recv().await {
                Ok(new_epoch) => {
                    log::info!("🔔 Epoch transition detected: epoch {}", new_epoch);
                    check_and_start_validator(
                        &datastore,
                        &node_peer_id,
                        new_epoch,
                        &keypair,
                        swarm.clone(),
                        consensus_tx.clone(),
                        consensus_rx.clone(),
                        control.clone(),
                        checkpoint_mode.clone(),
                    )
                    .await;
                }
                Err(e) => {
                    log::error!("Epoch transition channel closed: {}", e);
                    break;
                }
            }
        }
    });
}

/// Get the current mining epoch from the canonical chain tip.
async fn get_current_epoch(datastore: &Arc<Mutex<DatastoreManager>>) -> u64 {
    let blocks_per_epoch = {
        let ds = datastore.lock().await;
        ds.epoch_config().blocks_per_epoch.max(1)
    };
    let tip = get_chain_tip_index(datastore).await;
    mining_epoch_for_height(tip, blocks_per_epoch)
}

/// Check if this node should be a validator for the current epoch and start consensus if so.
async fn check_and_start_validator(
    datastore: &Arc<Mutex<DatastoreManager>>,
    node_peer_id: &str,
    current_epoch: u64,
    keypair: &Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: Arc<Mutex<Option<mpsc::Receiver<ConsensusMessage>>>>,
    control: SequencingControl,
    checkpoint_mode: CheckpointMode,
) {
    if current_epoch < 2 {
        control.participate.store(false, Ordering::SeqCst);
        return;
    }

    let validator_set = {
        let ds = datastore.lock().await;
        match get_validator_set_for_mining_epoch_hybrid_multi(&ds, current_epoch).await {
            Ok(Some(set)) => {
                log::info!(
                    "Validator set for epoch {}: {} validators",
                    current_epoch,
                    set.nominated_validators.len()
                );
                Some(set)
            }
            Ok(None) => {
                log::debug!(
                    "No validator set available for epoch {} yet (need epoch >= 2)",
                    current_epoch
                );
                None
            }
            Err(e) => {
                log::error!("Failed to get validator set for epoch {}: {}", current_epoch, e);
                None
            }
        }
    };

    let Some(validator_set) = validator_set else {
        control.participate.store(false, Ordering::SeqCst);
        return;
    };

    let validators_with_stakes = validator_set.get_active_validators_with_stakes();
    let validators: Vec<String> = validators_with_stakes.iter().map(|(p, _)| p.clone()).collect();
    let stakes: Vec<u64> = validators_with_stakes.iter().map(|(_, s)| *s).collect();

    control
        .committee_size
        .store(validators.len(), Ordering::SeqCst);

    if !validators.contains(&node_peer_id.to_string()) {
        control.participate.store(false, Ordering::SeqCst);
        log::info!(
            "This node is NOT in the validator set for epoch {}",
            current_epoch
        );
        return;
    }

    control.participate.store(true, Ordering::SeqCst);

    if control.started.load(Ordering::SeqCst) {
        log::info!(
            "🏛️  Still a validator for epoch {} — live loop already running (committee size {})",
            current_epoch,
            validators.len()
        );
        return;
    }

    log::info!(
        "🏛️  This node IS a validator for epoch {} - starting Shoal consensus",
        current_epoch
    );

    let my_index = validators
        .iter()
        .position(|v| v == node_peer_id)
        .expect("validator position in list");

    log::info!("📋 Validator index: {}/{}", my_index, validators.len());
    log::info!("📋 Active validators for epoch {}:", current_epoch);
    for (peer_id, stake) in &validators_with_stakes {
        let short_id = if peer_id.len() > 16 {
            &peer_id[..16]
        } else {
            peer_id
        };
        log::info!("   - {} (stake: {})", short_id, stake);
    }

    let total_stake: u64 = stakes.iter().sum();
    log::info!(
        "📊 Total stake: {}, My stake: {}",
        total_stake,
        stakes[my_index]
    );

    let Some(rx) = consensus_rx.lock().await.take() else {
        log::error!("Consensus receiver already taken; cannot start a second live loop");
        return;
    };

    match consensus::create_and_start_shoal_validator_weighted_with_epoch(
        validators,
        stakes,
        my_index,
        datastore.clone(),
        keypair.clone(),
        swarm,
        consensus_tx,
        rx,
        control.clone(),
        current_epoch,
        checkpoint_mode,
    )
    .await
    {
        Ok(()) => {
            control.started.store(true, Ordering::SeqCst);
            log::info!("✅ Hybrid consensus started for epoch {}", current_epoch);
        }
        Err(e) => log::error!("Failed to start hybrid consensus: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mining_epoch_for_height_uses_configured_blocks_per_epoch() {
        assert_eq!(mining_epoch_for_height(0, 40), 0);
        assert_eq!(mining_epoch_for_height(39, 40), 0);
        assert_eq!(mining_epoch_for_height(40, 40), 1);
        assert_eq!(mining_epoch_for_height(80, 40), 2);
        assert_eq!(mining_epoch_for_height(79, 40), 1);
        assert_eq!(mining_epoch_for_height(80, 100), 0);
        assert_eq!(mining_epoch_for_height(200, 100), 2);
        assert_eq!(mining_epoch_for_height(10, 0), 0);
    }

    #[test]
    fn n_minus_two_lookback_requires_epoch_two() {
        assert!(mining_epoch_for_height(79, 40) < 2);
        assert!(mining_epoch_for_height(80, 40) >= 2);
    }
}
