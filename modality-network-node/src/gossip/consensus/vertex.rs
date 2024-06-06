use crate::reqres::{Request, Response};
use crate::gossip::GossipMessage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::collections::HashMap;


pub const TOPIC: &str = "/consensus/vertex";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub sequencer: String,
    // other fields...
}

pub struct Node {
    pub storage: Option<Storage>,
    pub services: Services,
}

pub struct Storage {
    pub local_dag: Option<HashMap<String, String>>, // Example type, adjust as needed
}

pub struct Services {
    pub reqres: ReqResService,
}

pub struct ReqResService {
    // Define this service structure as needed
}

pub async fn handler(node: &Node, message: GossipMessage) -> Result<()> {
    // Check if node has storage and local_dag
    let local_dag = match &node.storage {
        Some(storage) => match &storage.local_dag {
            Some(local_dag) => local_dag,
            None => return Ok(()), // If no local_dag, do nothing
        },
        None => return Ok(()), // If no storage, do nothing
    };

    // Decode and parse the message data
    let vertex: Vertex = serde_json::from_str(&message.data)?;

    // TODO: Check if sender is this epoch sequencer
    // TODO: Actually sign vertex
    let signed_vertex = vertex.clone(); // Placeholder for signed vertex logic


    Ok(())
}
