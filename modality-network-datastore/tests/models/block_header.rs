#[cfg(test)]
mod tests {
    use anyhow::Result;
    use modality_network_datastore::models::block_header::{self, prelude::*};
    use modality_network_datastore::models::block::prelude::*;
    use modality_network_datastore::models::page::prelude::*;
    use modality_network_devnet::Devnet;

    #[tokio::test]
    async fn test_from_json() -> Result<()> {
        let block_header = BlockHeader::create_from_json(serde_json::json!({
            "block_id": 1,
            "peer_block_headers": {
                "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd": {
                    "block_id": 1,
                    "peer_id": "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
                    "prev_block_certs": "...",
                    "opening_sig": "...",
                    "cert": "...",
                }
            }
        }))?;
        assert_eq!(block_header.block_id, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_from_datastore() -> Result<()> {
        const NODE_COUNT: usize = 3;
        let mut ds = NetworkDatastore::create_in_memory()?;
        Devnet::setup_datastore_scribes(&mut ds, NODE_COUNT).await?;
        Devnet::add_fully_connected_empty_round(&mut ds).await?;
        Devnet::add_fully_connected_empty_round(&mut ds).await?;
        Devnet::add_fully_connected_empty_round(&mut ds).await?;
        
        let block_header = BlockHeader::create_from_datastore(&mut ds, 2).await?;
        let pbh: serde_json::Value = block_header.peer_block_headers;
        let peer_id = pbh.as_object().expect("should have peer_ids").keys().next().expect("no peer_id");
        let peer_block_header = pbh.get(peer_id).unwrap().as_object().expect("should have peer_block_header");
        assert_eq!(peer_block_header.get("block_id").unwrap(), 2);
        Ok(())
    }
}