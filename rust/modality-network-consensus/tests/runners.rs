#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use modality_network_consensus::communication::same_process::SameProcess;
    use modality_network_consensus::communication::Communication;
    use modality_network_consensus::election;
    use modality_network_consensus::runner::*;
    use modality_network_consensus::sequencing::static_authority::StaticAuthority;
    // use modality_network_datastore::models::block::Block;
    // use modality_network_datastore::Model;
    use modality_network_datastore::NetworkDatastore;
    use modality_network_devnet::Devnet;

    #[tokio::test]
    async fn test_runners() -> Result<()> {
        const NODE_COUNT: usize = 3;

        // Setup
        let scribes = Devnet::get_peerids(NODE_COUNT)?;
        let scribe_keypairs = Devnet::get_keypairs_dict(NODE_COUNT)?;
        let election = election::Election::RoundRobin(election::round_robin::RoundRobin::create());
        let sequencing = StaticAuthority::create(scribes.clone(), election).await;

        // Create datastore builder
        let mut ds = NetworkDatastore::create_in_memory()?;
        Devnet::setup_datastore_scribes(&mut ds, NODE_COUNT).await?;

        // Create the communication layer first
        let same_process = Arc::new(SameProcess::new());
        let shared_communication: Arc<Mutex<dyn Communication>> = Arc::new(Mutex::new((*same_process).clone()));

        // Create runners with Arc wrapping from the start
        let runner1 = Arc::new(Runner::create(RunnerProps {
            datastore: Arc::new(Mutex::new(ds.clone_to_memory().await?)),
            peerid: Some(scribes[0].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[0]].clone())),
            communication: Some(Arc::clone(&shared_communication)),
            sequencing: Arc::new(sequencing.clone()),
        }));

        let runner2 = Arc::new(Runner::create(RunnerProps {
            datastore: Arc::new(Mutex::new(ds.clone_to_memory().await?)),
            peerid: Some(scribes[1].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[1]].clone())),
            communication: Some(Arc::clone(&shared_communication)),
            sequencing: Arc::new(sequencing.clone()),
        }));

        let runner3 = Arc::new(Runner::create(RunnerProps {
            datastore: Arc::new(Mutex::new(ds.clone_to_memory().await?)),
            peerid: Some(scribes[2].clone()),
            keypair: Some(Arc::new(scribe_keypairs[&scribes[2]].clone())),
            communication: Some(Arc::clone(&shared_communication)),
            sequencing: Arc::new(sequencing.clone()),
        }));

        // Register runners properly using the SameProcess instance
        same_process
            .register_runner(&scribes[0], runner1.clone())
            .await;
        same_process
            .register_runner(&scribes[1], runner2.clone())
            .await;
        same_process
            .register_runner(&scribes[2], runner3.clone())
            .await;

        // Create mutable references for run_round
        let mut runner1 = runner1.as_ref().clone();
        let mut runner2 = runner2.as_ref().clone();
        let mut runner3 = runner3.as_ref().clone();

        futures::try_join!(
            runner1.run_round(None),
            runner2.run_round(None),
            runner3.run_round(None)
        ).expect("run_round failed");

        let current_round = runner1.datastore.lock().await.get_current_round().await?;
        assert_eq!(current_round, 3);
        let current_round = runner2.datastore.lock().await.get_current_round().await?;
        assert_eq!(current_round, 3);
        let current_round = runner3.datastore.lock().await.get_current_round().await?;
        assert_eq!(current_round, 3);

        Ok(())
    }
}
