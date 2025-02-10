#[cfg(test)]
mod tests {
    use anyhow::Result;
    use modality_network_devnet::Devnet;
    use modality_network_datastore::{Model, NetworkDatastore};
    use modality_network_datastore::models::block::Block;
    use modality_network_consensus::sequencing::static_authority::StaticAuthority;
    use modality_network_consensus::election;
    use modality_network_consensus::runner::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_event_handling() -> Result<()> {
        const NODE_COUNT: usize = 3;

        // Setup
        let scribes = Devnet::get_peerids(NODE_COUNT)?;
        let scribe_keypairs = Devnet::get_keypairs_dict(NODE_COUNT)?;
        let election = election::Election::RoundRobin(election::round_robin::RoundRobin::create());
        let sequencing = StaticAuthority::create(
            scribes.clone(),
            election
        ).await;

        // Create datastore builder
        let mut ds = NetworkDatastore::create_in_memory()?;
        Devnet::setup_datastore_scribes(&mut ds, NODE_COUNT).await?;

        // Create runners
        let mut runner1 = Runner::create(RunnerProps {
            datastore: Arc::new(ds.clone_to_memory().await?),
            peerid: Some(scribes[0].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[0]].clone())),
            // communication: Some(Arc::new(SameProcess::new())),
            communication: None,
            sequencing: Arc::new(sequencing.clone()),
        });

        let mut runner2 = Runner::create(RunnerProps {
            datastore: Arc::new(ds.clone_to_memory().await?),
            peerid: Some(scribes[1].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[1]].clone())),
            communication: None,
            sequencing: Arc::new(sequencing.clone()),
        });

        let mut runner3 = Runner::create(RunnerProps {
            datastore: Arc::new(ds.clone_to_memory().await?),
            peerid: Some(scribes[2].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[2]].clone())),
            communication: None,
            sequencing: Arc::new(sequencing.clone()),
        });

        // Round 2 from perspective of scribe 1
        let round_id = 2;
        let last_round_certs = runner1.datastore.get_timely_certs_at_round(round_id - 1).await?;
        let mut block = Block::create_from_json(serde_json::json!({
            "peer_id": scribes[0].to_string(),
            "round_id": round_id,
            "events": [],
            "prev_round_certs": serde_json::to_value(last_round_certs)?
        }))?;
        block.generate_sigs(&scribe_keypairs[&scribes[0]])?;
        block.save(&*runner1.datastore).await?;

        // // Process acks
        let ack= runner1.on_receive_draft_block(block.to_json_object()?).await?;
        runner1.on_receive_block_ack(ack).await?;

        let ack = runner2.on_receive_draft_block(block.to_json_object()?).await?;
        runner1.on_receive_block_ack(ack).await?;

        let ack = runner3.on_receive_draft_block(block.to_json_object()?).await?;
        runner1.on_receive_block_ack(ack).await?;

        // Reload and verify
        block.reload(&*runner1.datastore).await?;
        block.generate_cert(&scribe_keypairs[&scribes[0]])?;
        
        assert!(block.cert.is_some());
        assert_eq!(block.acks.len(), 3);
        assert!(block.validate_cert(3)?);

        // Test certified block handling
        let cert_block = runner2
            .on_receive_certified_block(block.to_json_object()?)
            .await?;
        assert!(cert_block.is_some());

        // Test invalid cert
        let mut invalid_block = block.to_json_object()?;
        invalid_block["cert"] = serde_json::Value::String("".to_string());
        let cert_block = runner2
            .on_receive_certified_block(invalid_block)
            .await?;
        assert!(cert_block.is_none());

        Ok(())
    }
}