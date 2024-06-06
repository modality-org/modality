use anyhow::Result;
use libp2p::gossipsub;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedSender;

pub mod consensus;

pub const PROTOCOL: &str = "/modality-network/gossip/0.0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub topic: String,
    pub data: String,
}

pub struct Behaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub topics: HashMap<gossipsub::TopicHash, UnboundedSender<GossipMessage>>,
}

impl Behaviour {
    pub fn new(gossipsub: gossipsub::Behaviour) -> Self {
        Behaviour {
            gossipsub,
            topics: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, topic: &str) -> Result<()> {
        let topic = gossipsub::IdentTopic::new(topic);
        self.gossipsub.subscribe(&topic)?;
        let (tx, mut rx) = unbounded_channel::<GossipMessage>();
        self.topics.insert(topic.hash(), tx);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                println!("Received message on topic {}: {}", message.topic, message.data);
            }
        });
        Ok(())
    }

    pub async fn handle_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message {
                propagation_source: _,
                message_id: _,
                message,
            } => {
                let topic = message.topic.to_string();
                let data = String::from_utf8_lossy(&message.data).to_string();
                let msg = GossipMessage { topic, data };
                if let Some(tx) = self.topics.get(&message.topic) {
                    if let Err(err) = tx.send(msg) {
                        eprintln!("Error sending message: {}", err);
                    }
                }
            }
            _ => {}
        }
    }
}
