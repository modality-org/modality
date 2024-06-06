use crate::gossip::GossipMessage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const TOPIC: &str = "/consensus/vertex_certificate";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexCertificate {
    // Define the fields as per your structure
}

pub struct Node {
    pub storage: Option<Arc<Mutex<Storage>>>,
    pub services: Services,
}

pub struct Storage {
    // Define this struct as per your requirements
}

pub struct Services {
    // Define this struct as per your requirements
}

pub async fn handler(node: &Node, message: GossipMessage) -> Result<()> {
    // Decode and parse the message data
    let text = message.data;
    let obj: VertexCertificate = serde_json::from_str(&text)?;

    // Log the event details
    // println!("Event: {:?}", message);
    println!("Text: {:?}", text);
    println!("Object: {:?}", obj);

    Ok(())
}
