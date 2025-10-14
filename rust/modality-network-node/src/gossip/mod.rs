use anyhow::Result;
use libp2p::gossipsub::{self, Message};
use tokio::sync::mpsc;

use modality_network_datastore::NetworkDatastore;
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

pub async fn handle_event(message: Message, datastore: &mut NetworkDatastore, consensus_tx: mpsc::Sender<ConsensusMessage>) -> Result<()> {
  log::info!("handling gossip: {:?}", message);
  let data = String::from_utf8_lossy(&message.data).to_string();
  let topic = message.topic.to_string();
  
  if &topic == consensus::block::draft::TOPIC {
    consensus::block::draft::handler(data, datastore, consensus_tx).await?;
  } else if &topic == consensus::block::cert::TOPIC {
    consensus::block::cert::handler(data, datastore, consensus_tx).await?;
  } else if &topic == miner::block::TOPIC {
    miner::block::handler(data, datastore).await?;
  } else {
    log::warn!("Unknown gossip topic: {}", topic);
  }
  
  Ok(())
}