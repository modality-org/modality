use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::communication::Communication;

use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

#[async_trait]
pub trait Node: Send + Sync {
    fn peerid(&self) -> &str;
    async fn on_receive_draft_block(&self, block_data: &Block) -> Result<()>;
    async fn on_receive_block_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_block_late_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_certified_block(&self, block_data: &Block) -> Result<()>;
    async fn on_fetch_peer_block_certified_block_request(&self, peer_id: &str, round_id: u64) -> Result<Option<Block>>;
}

pub struct SameProcess {
    nodes: HashMap<String, Arc<dyn Node>>,
    offline_nodes: Vec<String>,
}

impl SameProcess {
    pub fn new(nodes: HashMap<String, Arc<dyn Node>>) -> Self {
        Self {
            nodes,
            offline_nodes: Vec::new(),
        }
    }

    fn is_node_offline(&self, node_id: &str) -> bool {
        self.offline_nodes.contains(&node_id.to_string())
    }
}

#[async_trait]
impl Communication for SameProcess {
    async fn broadcast_draft_block(&self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if !self.is_node_offline(node.peerid()) {
                continue;
            }
            node.on_receive_draft_block(block_data).await?;
        }
        Ok(())
    }

    async fn send_block_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.is_node_offline(from) || self.is_node_offline(to) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_block_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn send_block_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.is_node_offline(from) || self.is_node_offline(to) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_block_late_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn broadcast_certified_page(&self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if !self.is_node_offline(node.peerid()) {
                continue;
            }
            node.on_receive_certified_block(block_data).await?;
        }
        Ok(())
    }

    async fn fetch_peer_block_certified_page(&self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>> {
        if self.is_node_offline(from) {
            return Ok(None);
        }

        if let Some(node) = self.nodes.get(to) {
            return node.on_fetch_peer_block_certified_block_request(peer_id, round_id).await;
        }
        Ok(None)
    }
}