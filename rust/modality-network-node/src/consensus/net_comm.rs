use anyhow::Result;

use modality_network_consensus::communication::Communication;
use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

use crate::node::Node;

pub struct NetComm {
    node: &'static mut Node
}

impl NetComm {
    pub fn new(node: &'static mut Node) -> Self {
        Self { node }
    }
}

impl Communication for NetComm {
    #[allow(unused)]
    async fn broadcast_draft_block(&self, from: &str, block_data: &Block) -> Result<()> {
        // if self.offline_nodes.contains(&from.to_string()) {
        //     return Ok(());
        // }

        // for node in self.nodes.values() {
        //     if self.offline_nodes.contains(&node.peerid().to_string()) {
        //         continue;
        //     }
        //     node.on_receive_draft_block(block_data).await?;
        // }
        Ok(())
    }

    #[allow(unused)]
    async fn send_block_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        // if self.offline_nodes.contains(&from.to_string()) || 
        //    self.offline_nodes.contains(&to.to_string()) {
        //     return Ok(());
        // }

        // if let Some(node) = self.nodes.get(to) {
        //     node.on_receive_block_ack(ack_data).await?;
        // }
        Ok(())
    }

    #[allow(unused)]
    async fn send_block_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        // if self.offline_nodes.contains(&from.to_string()) || 
        //    self.offline_nodes.contains(&to.to_string()) {
        //     return Ok(());
        // }

        // if let Some(node) = self.nodes.get(to) {
        //     node.on_receive_block_late_ack(ack_data).await?;
        // }
        Ok(())
    }

    #[allow(unused)]
    async fn broadcast_certified_page(&self, from: &str, block_data: &Block) -> Result<()> {
        // if self.offline_nodes.contains(&from.to_string()) {
        //     return Ok(());
        // }

        // for node in self.nodes.values() {
        //     if self.offline_nodes.contains(&node.peerid().to_string()) {
        //         continue;
        //     }
        //     node.on_receive_certified_block(block_data).await?;
        // }
        Ok(())
    }

    #[allow(unused)]
    async fn fetch_peer_block_certified_page(&self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>> {
        // if self.offline_nodes.contains(&from.to_string()) {
        //     return Ok(None);
        // }

        // if let Some(node) = self.nodes.get(to) {
        //     return node.on_fetch_peer_block_certified_block_request(peer_id, round_id).await;
        // }
        Ok(None)
    }
}