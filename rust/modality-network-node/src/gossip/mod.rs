use anyhow::Result;
use libp2p::gossipsub::{self, Message};

use crate::node::Node;

mod consensus;

pub async fn add_sequencer_event_listeners(node: &mut Node) -> Result<()> {
  let topic = gossipsub::IdentTopic::new(consensus::block::draft::TOPIC);
  node.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

  let topic = gossipsub::IdentTopic::new(consensus::block::cert::TOPIC);
  node.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

  Ok(())
}

pub async fn handle_event(node: &mut Node, message: Message) -> Result<()> {
  log::info!("handling gossip: {:?}", message);
  let data = String::from_utf8_lossy(&message.data).to_string();
  if &message.topic.to_string() == consensus::block::draft::TOPIC {
    consensus::block::draft::handler(node, data).await?;
  } else if &message.topic.to_string() == consensus::block::cert::TOPIC {
    consensus::block::cert::handler(node, data).await?;
  }
  Ok(())
}