use anyhow::Result;
use libp2p::gossipsub::{self, Message};

use modality_network_datastore::NetworkDatastore;

use crate::node::Node;

pub mod consensus;

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

pub async fn handle_event(message: Message, datastore: &mut NetworkDatastore) -> Result<()> {
  log::info!("handling gossip: {:?}", message);
  let data = String::from_utf8_lossy(&message.data).to_string();
  if &message.topic.to_string() == consensus::block::draft::TOPIC {
    consensus::block::draft::handler(data, datastore).await?;
  } else if &message.topic.to_string() == consensus::block::cert::TOPIC {
    consensus::block::cert::handler(data, datastore).await?;
  }
  Ok(())
}