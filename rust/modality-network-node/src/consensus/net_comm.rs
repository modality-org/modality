use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
// use std::sync::Mutex;

use modality_network_consensus::communication::Communication;
use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

use crate::node::Node;
use crate::gossip::consensus::block::draft::TOPIC as BLOCK_DRAFT_TOPIC;
use crate::gossip::consensus::block::cert::TOPIC as BLOCK_CERT_TOPIC;

pub struct NetComm {
    node: Arc<Mutex<Node>>,
    // node: Arc<Mutex<Node>>
}

impl NetComm {
    pub fn new(node: Node) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
        }
    }
}

#[async_trait::async_trait]
impl Communication for NetComm {
    async fn broadcast_draft_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        let mut node = self.node.lock().await;
        node.publish_gossip(BLOCK_DRAFT_TOPIC.to_string(), block_data.to_draft_json_string()).await?;
        Ok(())
    }

    #[allow(unused)]
    async fn send_block_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        // return await this.node.sendOrHandleRequest(
        //     to,
        //     "/consensus/block/ack",
        //     ack_data
        //   );
        Ok(())
    }

    #[allow(unused)]
    async fn send_block_late_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        // noop
        Ok(())
    }

    async fn broadcast_certified_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        let mut node = self.node.lock().await;
        node.publish_gossip(BLOCK_CERT_TOPIC.to_string(), block_data.to_draft_json_string()).await?;
        Ok(())
    }

    #[allow(unused)]
    async fn fetch_scribe_round_certified_block(&mut self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>> {
        // if (to === this.node.peerid) {
        //     return null;
        //   }
        //   const r = await this.node.sendOrHandleRequest(to, "/data/block", {
        //     round_id,
        //     peer_id,
        //   });
        //   if (r.data?.block) {
        //     return {block: SafeJSON.parse(r.data?.block)};
        //   }
        Ok(None)
    }
}