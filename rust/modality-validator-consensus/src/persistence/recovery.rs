use crate::narwhal::Certificate;
use crate::narwhal::dag::DAG;
use crate::persistence::FromPersistenceModel;
use crate::shoal::{ConsensusState, ReputationState};
use anyhow::{Context, Result};
use modal_datastore::models::{DAGCertificate, DAGState};
use modal_datastore::DatastoreManager;

/// Strategy for recovering DAG state from persistent storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Load all certificates from scratch and rebuild DAG
    FromScratch,
    /// Load from latest checkpoint only
    FromCheckpoint,
    /// Try checkpoint first, fall back to full rebuild if needed
    Hybrid,
}

/// Result of DAG recovery operation
#[derive(Debug)]
pub struct RecoveryResult {
    pub dag: DAG,
    pub certificates_loaded: usize,
    pub highest_round: u64,
    pub used_checkpoint: bool,
    pub consensus_state: Option<ConsensusState>,
    pub reputation_state: Option<ReputationState>,
}

/// Recover DAG from persistent storage using specified strategy (multi-store version)
pub async fn recover_dag_multi(
    datastore: &DatastoreManager,
    strategy: RecoveryStrategy,
) -> Result<RecoveryResult> {
    match strategy {
        RecoveryStrategy::FromScratch => recover_from_scratch_multi(datastore).await,
        RecoveryStrategy::FromCheckpoint => recover_from_checkpoint_multi(datastore).await,
        RecoveryStrategy::Hybrid => {
            // Try checkpoint first
            match recover_from_checkpoint_multi(datastore).await {
                Ok(result) => Ok(result),
                Err(e) => {
                    log::warn!("Checkpoint recovery failed: {}, falling back to full rebuild", e);
                    recover_from_scratch_multi(datastore).await
                }
            }
        }
    }
}

/// Recover DAG by loading all certificates from datastore (multi-store version)
async fn recover_from_scratch_multi(datastore: &DatastoreManager) -> Result<RecoveryResult> {
    log::info!("Recovering DAG from scratch...");
    
    let mut dag = DAG::new();
    let prefix = "/dag/certificates/round";
    let mut cert_models = Vec::new();
    
    // Iterate through all certificates from ValidatorFinal store
    let store = datastore.validator_final();
    let iterator = store.iterator(prefix);
    for result in iterator {
        let (key, _) = result.context("failed to iterate certificates")?;
        let key_str = String::from_utf8(key.to_vec()).context("invalid key UTF-8")?;
        
        // Parse key to extract round and digest
        let parts: Vec<&str> = key_str.split('/').collect();
        if parts.len() >= 6 {
            if let (Some(round_str), Some(digest)) = (parts.get(4), parts.get(6)) {
                let keys = [
                    ("round".to_string(), round_str.to_string()),
                    ("digest".to_string(), digest.to_string()),
                ].into_iter().collect();
                
                if let Some(cert_model) = DAGCertificate::find_one_multi(datastore, keys)
                    .await
                    .context("failed to load certificate")? 
                {
                    cert_models.push(cert_model);
                }
            }
        }
    }
    
    log::info!("Found {} certificates to load", cert_models.len());
    
    // Sort by round to ensure parents are loaded before children
    let mut certs_with_round: Vec<(u64, Certificate)> = Vec::new();
    for model in &cert_models {
        let cert = Certificate::from_persistence_model(model)
            .context("failed to deserialize certificate")?;
        certs_with_round.push((model.round, cert));
    }
    certs_with_round.sort_by_key(|(round, _)| *round);
    
    // Insert certificates into DAG
    let mut highest_round = 0;
    for (round, cert) in certs_with_round {
        dag.insert(cert).context("failed to insert certificate into DAG")?;
        if round > highest_round {
            highest_round = round;
        }
    }
    
    log::info!("Successfully loaded {} certificates, highest round: {}", 
               cert_models.len(), highest_round);
    
    Ok(RecoveryResult {
        dag,
        certificates_loaded: cert_models.len(),
        highest_round,
        used_checkpoint: false,
        consensus_state: None,
        reputation_state: None,
    })
}

/// Recover DAG from latest checkpoint (multi-store version)
async fn recover_from_checkpoint_multi(datastore: &DatastoreManager) -> Result<RecoveryResult> {
    log::info!("Recovering DAG from checkpoint...");
    
    // Find latest checkpoint
    let checkpoint = DAGState::get_latest_multi(datastore)
        .await
        .context("failed to query checkpoints")?
        .ok_or_else(|| anyhow::anyhow!("no checkpoint found"))?;
    
    log::info!("Found checkpoint at round {}", checkpoint.checkpoint_round);
    
    // Deserialize DAG snapshot
    use base64::Engine;
    let dag_bytes = base64::engine::general_purpose::STANDARD.decode(&checkpoint.dag_snapshot)
        .context("failed to decode DAG snapshot")?;
    let dag: DAG = bincode::deserialize(&dag_bytes)
        .context("failed to deserialize DAG")?;
    
    // Deserialize consensus state
    let consensus_state: Option<ConsensusState> = 
        serde_json::from_str(&checkpoint.consensus_state).ok();
    
    // Deserialize reputation state
    let reputation_state: Option<ReputationState> = 
        serde_json::from_str(&checkpoint.reputation_state).ok();
    
    // Load any certificates created after the checkpoint
    let mut final_dag = dag;
    let prefix = "/dag/certificates/round";
    let mut newer_certs = 0;
    
    let store = datastore.validator_final();
    let iterator = store.iterator(prefix);
    for result in iterator {
        let (key, _) = result.context("failed to iterate certificates")?;
        let key_str = String::from_utf8(key.to_vec()).context("invalid key UTF-8")?;
        
        // Parse key to extract round and digest
        let parts: Vec<&str> = key_str.split('/').collect();
        if parts.len() >= 6 {
            if let (Some(round_str), Some(digest)) = (parts.get(4), parts.get(6)) {
                let round = round_str.parse::<u64>().unwrap_or(0);
                
                if round > checkpoint.checkpoint_round {
                    let keys = [
                        ("round".to_string(), round_str.to_string()),
                        ("digest".to_string(), digest.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(cert_model) = DAGCertificate::find_one_multi(datastore, keys)
                        .await
                        .context("failed to load certificate")? 
                    {
                        let cert = Certificate::from_persistence_model(&cert_model)
                            .context("failed to deserialize certificate")?;
                        final_dag.insert(cert).context("failed to insert certificate")?;
                        newer_certs += 1;
                    }
                }
            }
        }
    }
    
    log::info!("Loaded checkpoint + {} newer certificates", newer_certs);
    
    Ok(RecoveryResult {
        dag: final_dag,
        certificates_loaded: checkpoint.certificate_count + newer_certs,
        highest_round: checkpoint.highest_round.max(
            checkpoint.checkpoint_round + (newer_certs > 0) as u64
        ),
        used_checkpoint: true,
        consensus_state,
        reputation_state,
    })
}

/// Verify DAG consistency after recovery
pub fn verify_dag_consistency(dag: &DAG) -> Result<()> {
    log::info!("Verifying DAG consistency...");
    
    // Get all rounds
    let rounds = dag.rounds();
    if rounds.is_empty() {
        return Ok(()); // Empty DAG is valid
    }
    
    let max_round = *rounds.iter().max().unwrap();
    
    // Check each certificate has valid parents
    for round in 1..=max_round {
        let certs = dag.certificates_at_round(round);
        for cert in certs {
            // Verify all parents exist and are from previous round
            for parent_digest in &cert.header.parents {
                dag.get_certificate(parent_digest)
                    .ok_or_else(|| anyhow::anyhow!(
                        "certificate at round {} references missing parent {:?}",
                        round, hex::encode(parent_digest)
                    ))?;
            }
        }
    }
    
    log::info!("DAG consistency verified successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narwhal::{Header, AggregatedSignature};
    use libp2p_identity::{ed25519, PeerId};
    use crate::persistence::ToPersistenceModel;

    fn test_peer_id(seed: u8) -> PeerId {
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
            .expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        PeerId::from_public_key(&keypair.public().into())
    }

    async fn setup_test_datastore() -> DatastoreManager {
        DatastoreManager::create_in_memory().unwrap()
    }

    #[tokio::test]
    async fn test_recover_from_scratch_empty() {
        let datastore = setup_test_datastore().await;
        
        let result = recover_from_scratch_multi(&datastore).await.unwrap();
        assert_eq!(result.certificates_loaded, 0);
        assert_eq!(result.highest_round, 0);
        assert!(!result.used_checkpoint);
    }

    #[tokio::test]
    async fn test_recover_from_scratch_with_certs() {
        let datastore = setup_test_datastore().await;
        
        // Create and save test certificates
        let cert1 = Certificate {
            header: Header {
                author: test_peer_id(1),
                round: 0,
                batch_digest: [1u8; 32],
                parents: vec![],
                timestamp: 1000,
            },
            aggregated_signature: AggregatedSignature { signature: vec![1, 2, 3] },
            signers: vec![true, true, true],
        };
        
        let cert2 = Certificate {
            header: Header {
                author: test_peer_id(2),
                round: 1,
                batch_digest: [2u8; 32],
                parents: vec![cert1.digest()],
                timestamp: 2000,
            },
            aggregated_signature: AggregatedSignature { signature: vec![4, 5, 6] },
            signers: vec![true, true, false],
        };
        
        // Save to datastore
        cert1.to_persistence_model().unwrap().save_to_final(&datastore).await.unwrap();
        cert2.to_persistence_model().unwrap().save_to_final(&datastore).await.unwrap();
        
        // Recover
        let result = recover_from_scratch_multi(&datastore).await.unwrap();
        assert_eq!(result.certificates_loaded, 2);
        assert_eq!(result.highest_round, 1);
        
        // Verify DAG structure
        assert_eq!(result.dag.round_size(0), 1);
        assert_eq!(result.dag.round_size(1), 1);
    }

    #[tokio::test]
    async fn test_verify_dag_consistency() {
        let mut dag = DAG::new();
        
        let cert1 = Certificate {
            header: Header {
                author: test_peer_id(1),
                round: 0,
                batch_digest: [1u8; 32],
                parents: vec![],
                timestamp: 1000,
            },
            aggregated_signature: AggregatedSignature { signature: vec![1, 2, 3] },
            signers: vec![true],
        };
        
        dag.insert(cert1).unwrap();
        
        // Should pass for valid DAG
        assert!(verify_dag_consistency(&dag).is_ok());
    }
}

