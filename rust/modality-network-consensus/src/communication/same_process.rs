use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::Weak;

use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;
use crate::communication::Communication;

#[async_trait]
pub trait ConsensusRunner: Send + Sync {
    fn peerid(&self) -> &str;
    async fn on_receive_draft_block(&self, block_data: &Block) -> Result<()>;
    async fn on_receive_block_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_block_late_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_certified_block(&self, block_data: &Block) -> Result<()>;
    async fn on_fetch_scribe_round_certified_block_request(&self, peer_id: &str, round_id: u64) -> Result<Option<Block>>;
}

pub struct SameProcess {
    consensus_runners: Mutex<HashMap<String, Weak<dyn ConsensusRunner>>>,
    offline_nodes: Vec<String>,
}

impl SameProcess {
    pub fn new() -> Self {
        Self {
            consensus_runners: Mutex::new(HashMap::new()),
            offline_nodes: Vec::new(),
        }
    }

    fn is_node_offline(&self, node_id: &str) -> bool {
        self.offline_nodes.contains(&node_id.to_string())
    }

    pub async fn register_runner(&self, peer_id: &str, consensus_runner: Arc<dyn ConsensusRunner>) {
        self.consensus_runners
            .lock()
            .await
            .insert(peer_id.to_string(), Arc::downgrade(&consensus_runner));
    }
}

#[async_trait]
impl Communication for SameProcess {
    async fn broadcast_draft_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        let peer_runners: Vec<Arc<dyn ConsensusRunner>> = {
            let runners = self.consensus_runners.lock().await;
            runners
                .iter()
                .filter(|(peer_id, _)| peer_id != &from)
                .filter_map(|(_, weak_runner)| weak_runner.upgrade())
                .collect()
        };

        for runner in peer_runners {
            runner.on_receive_draft_block(block_data).await?;
        }
        Ok(())
    }

    async fn send_block_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.is_node_offline(from) || self.is_node_offline(to) {
            return Ok(());
        }

        let consensus_runner = {
            let runners = self.consensus_runners.lock().await;
            runners.get(to).and_then(|weak| weak.upgrade())
        };

        if let Some(runner) = consensus_runner {
            runner.on_receive_block_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn send_block_late_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.is_node_offline(from) || self.is_node_offline(to) {
            return Ok(());
        }

        let consensus_runner = {
            let runners = self.consensus_runners.lock().await;
            runners.get(to).and_then(|weak| weak.upgrade())
        };

        if let Some(runner) = consensus_runner {
            runner.on_receive_block_late_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn broadcast_certified_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        let peer_runners: Vec<Arc<dyn ConsensusRunner>> = {
            let runners = self.consensus_runners.lock().await;
            runners
                .iter()
                .filter_map(|(_, weak_runner)| weak_runner.upgrade())
                .filter(|runner| !self.is_node_offline(runner.peerid()))
                .collect()
        };

        for runner in peer_runners {
            runner.on_receive_certified_block(block_data).await?;
        }
        Ok(())
    }

    async fn fetch_scribe_round_certified_block(
        &mut self,
        from: &str,
        to: &str,
        peer_id: &str,
        round_id: u64,
    ) -> Result<Option<Block>> {
        if self.is_node_offline(from) {
            return Ok(None);
        }

        let consensus_runner = {
            let runners = self.consensus_runners.lock().await;
            runners.get(to).and_then(|weak| weak.upgrade())
        };

        if let Some(runner) = consensus_runner {
            return runner.on_fetch_scribe_round_certified_block_request(peer_id, round_id).await;
        }
        Ok(None)
    }
}