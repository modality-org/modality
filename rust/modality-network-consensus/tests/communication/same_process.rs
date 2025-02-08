
#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use async_trait::async_trait;
    use anyhow::Result;
    use modality_network_consensus::communication::Communication;
    use modality_network_consensus::communication::same_process::*;
    use std::sync::Arc;
    use modality_network_datastore::models::block::Block;
    use modality_network_datastore::models::block::Ack;

    struct MockNode {
        peerid: String,
    }

    impl MockNode {
        fn new(peerid: &str) -> Self {
            Self {
                peerid: peerid.to_string(),
            }
        }
    }

    #[async_trait]
    impl Node for MockNode {
        fn peerid(&self) -> &str {
            &self.peerid
        }

        async fn on_receive_draft_block(&self, _block_data: &Block) -> Result<()> {
            Ok(())
        }

        async fn on_receive_block_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_block_late_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_certified_block(&self, _block_data: &Block) -> Result<()> {
            Ok(())
        }

        async fn on_fetch_peer_block_certified_block_request(&self, _peer_id: &str, _block_id: u64) -> Result<Option<Block>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_same_process_basic() -> Result<()> {
        let mut nodes = HashMap::new();
        let node = Arc::new(MockNode::new("test")) as Arc<dyn Node>;
        nodes.insert("test".to_string(), node);
        
        let same_process = SameProcess::new(nodes);
        let block_data = Block::create_from_json(serde_json::json!({
            "peer_id": "",
            "round_id": 1,
            "events": []
        }))?;
        
        same_process.broadcast_draft_page("test", &block_data).await?;
        Ok(())
    }
}