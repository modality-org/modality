#[cfg(test)]
mod tests {
    use modality_utils::keypair::Keypair;
    use anyhow::{Result};
    use modality_network_datastore::NetworkDatastore;
    use modality_network_datastore::Model;
    use modality_network_datastore::models::block::Block;

    #[tokio::test]
    async fn test_page() -> Result<()> {
        let datastore = NetworkDatastore::create_in_memory()?;

        let node1_keypair = Keypair::generate()?;
        let node1_pubkey = node1_keypair.as_public_address();

        let node2_keypair = Keypair::generate()?;
        let _node2_pubkey = node2_keypair.as_public_address();

        let mut b1 = Block::create_from_json(serde_json::json!({
            "peer_id": node1_pubkey,
            "round_id": 1,
            "events": []
        }))?;

        b1.add_event(serde_json::json!({"data": "data1"}));
        b1.add_event(serde_json::json!({"data": "data2"}));
        assert_eq!(b1.events.len(), 2);

        let sig1 = b1.generate_sigs(&node1_keypair)?;
        let result = b1.validate_sigs()?;
        assert!(result);

        let mut b1empty = Block::create_from_json(serde_json::json!({
            "peer_id": node1_pubkey,
            "round_id": 1,
            "events": []
        }))?;
        let sig1empty = b1empty.generate_sigs(&node1_keypair)?;
        assert_ne!(sig1, sig1empty);

        // ack self
        let ack1 = b1.generate_ack(&node1_keypair)?;
        b1.add_ack(ack1)?;
        let result = b1.count_valid_acks()?;
        assert_eq!(result, 1);
        let result = b1.validate_acks()?;
        assert!(result);

        // other acks
        let ack2 = b1.generate_ack(&node2_keypair)?;
        b1.add_ack(ack2.clone())?;
        assert_eq!(b1.acks.get(&ack2.acker), Some(&ack2.acker_sig));
        let result = b1.validate_acks()?;
        assert!(result);
        let result = b1.count_valid_acks()?;
        assert_eq!(result, 2);

        b1.generate_cert(&node1_keypair)?;
        assert!(b1.cert.is_some());
        let result = b1.validate_cert(2)?;
        assert!(result);
        b1.save(&datastore).await?;

        let result = b1.get_id();
        assert_eq!(result, format!("/blocks/round/1/peer/{}", node1_pubkey));
        let b1r = Block::find_one(&datastore, [
            ("round_id".to_string(), "1".to_string()),
            ("peer_id".to_string(), node1_pubkey.clone())
        ].into_iter().collect()).await?.unwrap();
        assert_eq!(b1r.cert, b1.cert);

        let r1blocks = Block::find_all_in_round(&datastore, 1).await?;
        assert_eq!(r1blocks.len(), 1);

        Ok(())
    }
}