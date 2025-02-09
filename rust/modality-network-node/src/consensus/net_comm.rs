use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use modality_network_consensus::communication::Communication;

use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

pub struct NetComm {
    nodes: HashMap<String, Arc<dyn Node>>,
    offline_nodes: Vec<String>,
}

#[async_trait]
pub trait Node: Send + Sync {
    fn peerid(&self) -> &str;
    async fn on_receive_draft_block(&self, block_data: &Block) -> Result<()>;
    async fn on_receive_block_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_block_late_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_certified_block(&self, block_data: &Block) -> Result<()>;
    async fn on_fetch_peer_block_certified_block_request(&self, peer_id: &str, round_id: u64) -> Result<Option<Block>>;
}

impl NetComm {
    pub fn new(nodes: HashMap<String, Arc<dyn Node>>) -> Self {
        Self {
            nodes,
            offline_nodes: Vec::new(),
        }
    }
}

#[async_trait]
impl Communication for NetComm {
    async fn broadcast_draft_page(&self, from: &str, block_data: &Block) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if self.offline_nodes.contains(&node.peerid().to_string()) {
                continue;
            }
            node.on_receive_draft_block(block_data).await?;
        }
        Ok(())
    }

    async fn send_block_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) || 
           self.offline_nodes.contains(&to.to_string()) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_block_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn send_block_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) || 
           self.offline_nodes.contains(&to.to_string()) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_block_late_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn broadcast_certified_page(&self, from: &str, block_data: &Block) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if self.offline_nodes.contains(&node.peerid().to_string()) {
                continue;
            }
            node.on_receive_certified_block(block_data).await?;
        }
        Ok(())
    }

    async fn fetch_peer_block_certified_page(&self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(None);
        }

        if let Some(node) = self.nodes.get(to) {
            return node.on_fetch_peer_block_certified_block_request(peer_id, round_id).await;
        }
        Ok(None)
    }
}