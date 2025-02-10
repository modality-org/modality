
#[cfg(test)]
mod tests {
    use anyhow::Result;
    use modality_network_consensus::communication::Communication;
    use modality_network_consensus::communication::same_process::*;
    use modality_network_consensus::runner::ConsensusRunner;
    use std::sync::Arc;
    use modality_network_datastore::models::block::Block;
    use modality_network_datastore::models::block::Ack;

    struct MockConsensusRunner {
        peerid: String,
    }

    impl MockConsensusRunner {
        fn new(peerid: &str) -> Self {
            Self {
                peerid: peerid.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ConsensusRunner for MockConsensusRunner {
        fn peerid(&self) -> &str {
            &self.peerid
        }

        async fn on_receive_draft_block(&self, _block_data: &Block) -> Result<Option<Ack>> {
            return Err(anyhow::anyhow!("error"))
        }

        async fn on_receive_block_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_block_late_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_certified_block(&self, _block_data: &Block) -> Result<Option<Block>> {
            return Err(anyhow::anyhow!("error"))
        }

        async fn on_fetch_scribe_round_certified_block_request(&self, _peer_id: &str, _block_id: u64) -> Result<Option<Block>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_same_process_basic() -> Result<()> {
        let node = Arc::new(MockConsensusRunner::new("test")) as Arc<dyn ConsensusRunner>;
        let mut same_process = SameProcess::new();
        same_process.register_runner("test", node).await;
        let block_data = Block::create_from_json(serde_json::json!({
            "peer_id": "",
            "round_id": 1,
            "events": []
        }))?;
        
        same_process.broadcast_draft_block("test", &block_data).await?;
        Ok(())
    }
}