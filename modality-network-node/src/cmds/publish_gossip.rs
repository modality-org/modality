use crate::config_file::Config;
use crate::identity_utils::identity_from_private_key;
use crate::swarm::create_swarm;
use anyhow::Result;
use async_std::task;
use futures::StreamExt;
use libp2p::{
    gossipsub::{Gossipsub, GossipsubConfig, IdentTopic, MessageAuthenticity, ValidationMode},
    identity, PeerId, Swarm,
};
use std::time::Duration;
use tokio::time::sleep;

pub async fn publish_gossip(
    config: &str,
    keypair_path: Option<&str>,
    listen: &str,
    storage: Option<&str>,
    topic: &str,
    message: &str,
) -> Result<()> {
    let config: Config = config_file::read_or_create_config(config)?;
    let keypair_path = keypair_path.unwrap_or(&config.keypair.unwrap());
    let private_key = std::fs::read_to_string(keypair_path)?;
    let node_keypair = identity_from_private_key(private_key).await?;
    let node_peer_id = PeerId::from(node_keypair.public());

    let gossipsub_config = GossipsubConfig::default();
    let gossipsub = Gossipsub::new(
        MessageAuthenticity::Signed(node_keypair.clone()),
        gossipsub_config,
    )?;

    let topic = IdentTopic::new(topic);
    gossipsub.subscribe(&topic)?;

    let mut swarm = create_swarm(node_keypair.clone(), gossipsub, listen.parse()?).await?;

    let listen_ma = listen.parse().expect("Failed to parse listen address");
    swarm.listen_on(listen_ma)?;

    // Wait for peers to connect
    while swarm.behaviour().gossipsub().all_peers().is_empty() {
        sleep(Duration::from_secs(1)).await;
    }

    // Publish the message
    let message_bytes = message.as_bytes();
    swarm.behaviour_mut().gossipsub().publish(topic.clone(), message_bytes)?;

    // Give it some time to propagate
    sleep(Duration::from_secs(1)).await;

    // Stop the node
    swarm.close_all_connections().await;

    Ok(())
}
