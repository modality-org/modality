use anyhow::Result;
use libp2p::gossipsub::{self, Message};
use tokio::sync::mpsc;

use modal_datastore::NetworkDatastore;
use modality_network_consensus::communication::Message as ConsensusMessage;

use crate::node::Node;

pub mod consensus;
pub mod miner;

pub async fn add_sequencer_event_listeners(node: &mut Node) -> Result<()> {
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
    log::info!("Subscribed to miner block gossip topic: {}", miner::block::TOPIC);
  }

  Ok(())
}

pub async fn handle_event(
    message: Message, 
    datastore: std::sync::Arc<tokio::sync::Mutex<NetworkDatastore>>,
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
  
  if &topic == consensus::block::draft::TOPIC {
    let mut ds = datastore.lock().await;
    consensus::block::draft::handler(data, &mut ds, consensus_tx).await?;
  } else if &topic == consensus::block::cert::TOPIC {
    let mut ds = datastore.lock().await;
    consensus::block::cert::handler(data, &mut ds, consensus_tx).await?;
  } else if &topic == miner::block::TOPIC {
    miner::block::handler(data, source_peer, datastore, sync_request_tx, mining_update_tx, bootstrappers, minimum_block_timestamp).await?;
  } else {
    log::warn!("Unknown gossip topic: {}", topic);
  }
  
  Ok(())
}