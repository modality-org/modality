#[cfg(test)]
mod tests {
    use anyhow::Result;
    use modality_network_datastore::models::block_header::{self, prelude::*};
    use modality_network_datastore::models::block::prelude::*;
    use modality_network_devnet::Devnet;

    #[tokio::test]
    async fn test_from_json() -> Result<()> {
        let block_header = BlockHeader::create_from_json(serde_json::json!({
            "round_id": 1,
            "peer_id": "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
            "prev_round_certs": {"12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd": "..."},
            "opening_sig": "...",
            "cert": "...",
        }))?;
        assert_eq!(block_header.round_id, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_from_datastore() -> Result<()> {
        const NODE_COUNT: usize = 3;
        let mut ds = NetworkDatastore::create_in_memory()?;
        Devnet::setup_datastore_scribes(&mut ds, NODE_COUNT).await?;
        Devnet::add_fully_connected_empty_round(&mut ds, NODE_COUNT).await?;
        Devnet::add_fully_connected_empty_round(&mut ds, NODE_COUNT).await?;
        Devnet::add_fully_connected_empty_round(&mut ds, NODE_COUNT).await?;
        
        BlockHeader::dervive_all_in_round(&mut ds, 2).await?;
        let block_headers = BlockHeader::find_all_in_round(&mut ds, 2).await?;
        assert_eq!(block_headers.len(), NODE_COUNT);
        Ok(())
    }
}