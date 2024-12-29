
#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use async_trait::async_trait;
    use anyhow::Result;
    use modality_network_consensus::communication::Communication;
    use modality_network_consensus::communication::same_process::*;
    use std::sync::Arc;
    use modality_network_datastore::models::page::Page;
    use modality_network_datastore::models::page::Ack;

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

        async fn on_receive_draft_page(&self, _page_data: &Page) -> Result<()> {
            Ok(())
        }

        async fn on_receive_page_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_page_late_ack(&self, _ack_data: &Ack) -> Result<()> {
            Ok(())
        }

        async fn on_receive_certified_page(&self, _page_data: &Page) -> Result<()> {
            Ok(())
        }

        async fn on_fetch_peer_block_certified_page_request(&self, _peer_id: &str, _block_id: u64) -> Result<Option<Page>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_same_process_basic() -> Result<()> {
        let mut nodes = HashMap::new();
        let node = Arc::new(MockNode::new("test")) as Arc<dyn Node>;
        nodes.insert("test".to_string(), node);
        
        let same_process = SameProcess::new(nodes);
        let page_data = Page::create_from_json(serde_json::json!({
            "peer_id": "",
            "block_id": 1,
            "events": []
        }))?;
        
        same_process.broadcast_draft_page("test", &page_data).await?;
        Ok(())
    }
}