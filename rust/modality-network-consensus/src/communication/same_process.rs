use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::mpsc;

use modality_network_datastore::models::block::Ack;
use modality_network_datastore::models::Block;
// use modality_network_datastore::Model;
// use modality_network_datastore::NetworkDatastore;

use crate::communication::Message;
use crate::communication::Communication;
use crate::runner::ConsensusRunner;

#[derive(Clone)]
pub struct SameProcess {
    pub consensus_runners: Arc<Mutex<HashMap<String, Weak<dyn ConsensusRunner>>>>,
    offline_nodes: Vec<String>,
    message_queue: mpsc::UnboundedSender<Message>,
}

impl SameProcess {
    pub fn new() -> Self {
        let consensus_runners = Arc::new(Mutex::new(HashMap::new()));
        let runners_clone = Arc::clone(&consensus_runners);
        let (message_queue, mut queue_consumer) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(msg) = queue_consumer.recv().await {
                match msg {
                    Message::DraftBlock { from: _, to, block } => {
                        if let Some(runner) = Self::get_runner(&runners_clone, &to) {
                            let _ = runner.on_receive_draft_block(&block).await;
                        }
                    }
                    Message::BlockAck { from: _, to, ack } => {
                        if let Some(runner) = Self::get_runner(&runners_clone, &to) {
                            let _ = runner.on_receive_block_ack(&ack).await;
                        }
                    }
                    Message::BlockLateAck { from: _, to, ack } => {
                        if let Some(runner) = Self::get_runner(&runners_clone, &to) {
                            let _ = runner.on_receive_block_late_ack(&ack).await;
                        }
                    }
                    Message::CertifiedBlock { from: _, to, block } => {
                        if let Some(runner) = Self::get_runner(&runners_clone, &to) {
                            let _ = runner.on_receive_certified_block(&block).await;
                        }
                    }
                }
            }
        });

        Self {
            consensus_runners,
            offline_nodes: Vec::new(),
            message_queue,
        }
    }

    // Helper to get a single runner
    fn get_runner(
        runners: &Arc<Mutex<HashMap<String, Weak<dyn ConsensusRunner>>>>,
        peer_id: &str,
    ) -> Option<Arc<dyn ConsensusRunner>> {
        let runners = runners.lock().unwrap();
        runners.get(peer_id).and_then(|weak| weak.upgrade())
    }

    // Helper to get all runners except one
    // fn get_all_runners_except(
    //     runners: &Arc<Mutex<HashMap<String, Weak<dyn ConsensusRunner>>>>,
    //     except: &str,
    // ) -> Vec<Arc<dyn ConsensusRunner>> {
    //     let runners = runners.lock().unwrap();
    //     runners
    //         .iter()
    //         .filter(|(peer_id, _)| *peer_id != except)
    //         .filter_map(|(_, weak)| weak.upgrade())
    //         .collect()
    // }

    pub async fn register_runner(&self, peer_id: &str, consensus_runner: Arc<dyn ConsensusRunner>) {
        let mut runners = self.consensus_runners.lock().unwrap();
        runners.insert(peer_id.to_string(), Arc::downgrade(&consensus_runner));
    }

    fn is_node_offline(&self, node_id: &str) -> bool {
        self.offline_nodes.contains(&node_id.to_string())
    }
}

#[async_trait::async_trait]
impl Communication for SameProcess {
    async fn broadcast_draft_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        // Get list of peers to send to
        let peer_ids: Vec<String> = {
            let runners = self.consensus_runners.lock().unwrap();
            runners
                .iter()
                // .filter(|(peer_id, _)| *peer_id != from)
                .map(|(peer_id, _)| peer_id.clone())
                .collect()
        };

        // Send a message for each peer
        for to in peer_ids {
            self.message_queue.send(Message::DraftBlock {
                from: from.to_string(),
                to,
                block: block_data.clone(),
            })?;
        }

        Ok(())
    }

    async fn send_block_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if !self.is_node_offline(from) && !self.is_node_offline(to) {
            self.message_queue.send(Message::BlockAck {
                from: from.to_string(),
                to: to.to_string(),
                ack: ack_data.clone(),
            })?;
        }
        Ok(())
    }

    async fn send_block_late_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if !self.is_node_offline(from) && !self.is_node_offline(to) {
            self.message_queue.send(Message::BlockLateAck {
                from: from.to_string(),
                to: to.to_string(),
                ack: ack_data.clone(),
            })?;
        }
        Ok(())
    }

    async fn broadcast_certified_block(&mut self, from: &str, block_data: &Block) -> Result<()> {
        if self.is_node_offline(from) {
            return Ok(());
        }

        let peer_ids: Vec<String> = {
            let runners = self.consensus_runners.lock().unwrap();
            runners
                .iter()
                // .filter(|(peer_id, _)| *peer_id != from)
                .map(|(peer_id, _)| peer_id.clone())
                .collect()
        };

        for to in peer_ids {
            self.message_queue.send(Message::CertifiedBlock {
                from: from.to_string(),
                to: to.to_string(),
                block: block_data.clone(),
            })?;
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

        if let Some(runner) = Self::get_runner(&self.consensus_runners, to) {
            runner
                .on_fetch_scribe_round_certified_block_request(peer_id, round_id)
                .await
        } else {
            Ok(None)
        }
    }
}
