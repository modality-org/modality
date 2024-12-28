#[cfg(test)]
mod tests {
    use anyhow::{Result};
    use modality_network_datastore::NetworkDatastore;
    use modality_network_datastore::models::block::Block;
    use modality_network_datastore::models::block::prelude::*;
    
    #[tokio::test]
    async fn test_block() -> Result<()> {
        let datastore = NetworkDatastore::create_in_memory()?;
        
        let block = Block::create_from_json(serde_json::json!({"block_id": 1}))?;
        block.save(&datastore).await?;
        
        let block = Block::create_from_json(serde_json::json!({"block_id": 2}))?;
        block.save(&datastore).await?;
        
        let block = Block::create_from_json(serde_json::json!({"block_id": 3}))?;
        block.save(&datastore).await?;
        
        let max_block = Block::find_max_id(&datastore).await?;
        assert_eq!(max_block, Some(3));

        Ok(())
    }

    #[test]
    fn test_add_remove_scribe() -> Result<()> {
        let mut block = Block::create_from_json(serde_json::json!({"block_id": 1}))?;
        
        block.add_scribe("peer1".to_string());
        assert_eq!(block.scribes, vec!["peer1"]);

        block.add_scribe("peer2".to_string());
        assert_eq!(block.scribes, vec!["peer1", "peer2"]);

        block.remove_scribe("peer1");
        assert_eq!(block.scribes, vec!["peer2"]);

        Ok(())
    }
}