use anyhow::Result;
use libp2p::gossipsub::{self, Message};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use modality_datastore::DatastoreManager;
use modality_validator_consensus::communication::Message as ConsensusMessage;

use crate::node::Node;

pub mod consensus;
pub mod miner;

pub async fn add_validator_event_listeners(node: &mut Node) -> Result<()> {
    {
        let mut swarm = node.swarm.lock().await;

        let topic = gossipsub::IdentTopic::new(consensus::block::draft::TOPIC);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        let topic = gossipsub::IdentTopic::new(consensus::block::cert::TOPIC);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    }

    Ok(())
}

pub async fn add_miner_event_listeners(node: &mut Node) -> Result<()> {
    {
        let mut swarm = node.swarm.lock().await;

        let topic = gossipsub::IdentTopic::new(miner::block::TOPIC);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        log::info!(
            "Subscribed to miner block gossip topic: {}",
            miner::block::TOPIC
        );
    }

    Ok(())
}

pub async fn handle_event(
    message: Message,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    sync_request_tx: Option<mpsc::UnboundedSender<(libp2p::PeerId, String)>>,
    mining_update_tx: Option<mpsc::UnboundedSender<u64>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
    minimum_block_timestamp: Option<i64>,
) -> Result<()> {
    log::info!("handling gossip: {:?}", message);
    let data = String::from_utf8_lossy(&message.data).to_string();
    let topic = message.topic.to_string();
    let source_peer = message.source;

    // Do not hold the datastore lock across the consensus-channel send.
    // The Shoal loop is the channel's only consumer and also needs that
    // lock. Holding it here while the channel is full wedges the process:
    // status, sync, and the swarm accept loop all stop.
    if topic == consensus::block::draft::TOPIC {
        consensus::block::draft::handler(data, consensus_tx).await?;
    } else if topic == consensus::block::cert::TOPIC {
        consensus::block::cert::handler(data, consensus_tx).await?;
    } else if topic == miner::block::TOPIC {
        miner::block::handler(
            data,
            source_peer,
            datastore_manager,
            sync_request_tx,
            mining_update_tx,
            bootstrappers,
            minimum_block_timestamp,
        )
        .await?;
    } else {
        log::warn!("Unknown gossip topic: {}", topic);
    }

    Ok(())
}
