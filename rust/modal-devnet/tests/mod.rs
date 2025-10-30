#[cfg(test)]
mod tests {
    use anyhow::Result;
    use modal_devnet::{Devnet, KEYPAIRS};

    // Helper function to create a test instance
    fn setup() -> Devnet {
        Devnet::new()
    }

    #[tokio::test]
    async fn test_get_keypairs() -> Result<()> {
        let common: Devnet = setup();
        
        // Test getting all keypairs
        let all_keypairs = common.get_keypairs(None).await?;
        assert!(!all_keypairs.is_empty());
        
        // Test getting specific number of keypairs
        let count = 2;
        let some_keypairs = common.get_keypairs(Some(count)).await?;
        assert_eq!(some_keypairs.len(), count);
        
        // Test requesting too many keypairs
        let too_many = KEYPAIRS.len() + 1;
        assert!(common.get_keypairs(Some(too_many)).await.is_err());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_get_keypair_by_index() -> Result<()> {
        let common = setup();
        
        // Test getting first keypair
        let first_keypair = common.get_keypair_by_index(0).await?;
        assert!(!first_keypair.as_public_key_id().is_empty());  // Assuming Keypair has an id() method
        
        // Test invalid index
        assert!(common.get_keypair_by_index(KEYPAIRS.len()).await.is_err());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_get_peerids() -> Result<()> {
        // Test getting specific number of peer IDs
        let count = 2;
        let some_peers = Devnet::get_peerids(count)?;
        assert_eq!(some_peers.len(), count);
        
        // Ensure peer IDs are strings
        for peer_id in some_peers {
            assert!(!peer_id.is_empty());
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_peerid_of_and_index_of() {
        let common = setup();
        
        // Get a peer ID at index 0
        let peer_id = common.peerid_of(0).expect("Should have peer ID at index 0");
        
        // Find the index of that peer ID
        let index = common.index_of(&peer_id).expect("Should find index of peer ID");
        
        // Verify we got back to the same index
        assert_eq!(index, 0);
        
        // Test invalid index
        assert!(common.peerid_of(KEYPAIRS.len()).is_none());
        
        // Test invalid peer ID
        assert!(common.index_of("invalid-peer-id").is_none());
    }

    #[tokio::test]
    async fn test_get_keypair_for() -> Result<()> {
        let common = setup();
        
        // Get a valid peer ID first
        let peer_id = common.peerid_of(0).expect("Should have peer ID at index 0");
        
        // Test getting keypair for valid ID
        let keypair = common.get_keypair_for(&peer_id).await?;
        assert_eq!(keypair.as_public_key_id(), peer_id);  // Assuming Keypair has an id() method
        
        // Test getting keypair for invalid ID
        assert!(common.get_keypair_for("invalid-id").await.is_err());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_get_keypairs_dict() -> Result<()> {        
        // Test getting specific number of keypairs
        let count = 2;
        let some_keypairs = Devnet::get_keypairs_dict(count)?;
        assert_eq!(some_keypairs.len(), count);
        
        // Verify the keys match the keypair IDs
        for (key, keypair) in some_keypairs.iter() {
            assert_eq!(key, &keypair.as_public_key_id());
        }
        
        // Test requesting too many keypairs
        let too_many = KEYPAIRS.len() + 1;
        assert!(Devnet::get_keypairs_dict(too_many).is_err());
        
        Ok(())
    }
}