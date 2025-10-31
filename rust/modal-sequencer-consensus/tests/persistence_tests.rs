use modal_sequencer_consensus::narwhal::{
    AggregatedSignature, Batch, Certificate, Committee, Header, Transaction, Validator,
};
use modal_sequencer_consensus::narwhal::dag::DAG;
use modal_sequencer_consensus::persistence::{
    FromPersistenceModel, ToPersistenceModel, recovery::{recover_dag, RecoveryStrategy, verify_dag_consistency},
};
use modal_sequencer_consensus::shoal::{ConsensusState, ReputationState, ReputationConfig};
use modal_datastore::{NetworkDatastore, Model};
use modal_datastore::models::{DAGCertificate, DAGBatch, DAGState, ConsensusMetadata};
use libp2p_identity::{ed25519, PeerId};
use std::net::SocketAddr;
use tempfile::TempDir;

fn test_peer_id(seed: u8) -> PeerId {
    let mut secret_bytes = [0u8; 32];
    secret_bytes[0] = seed;
    let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
        .expect("valid secret key");
    let keypair = ed25519::Keypair::from(secret);
    PeerId::from_public_key(&keypair.public().into())
}

async fn setup_test_datastore() -> (NetworkDatastore, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let datastore = NetworkDatastore::new(temp_dir.path()).unwrap();
    (datastore, temp_dir)
}

fn create_test_committee(n: usize) -> Committee {
    let mut validators = std::collections::HashMap::new();
    let mut validator_order = Vec::new();
    
    for i in 0..n {
        let peer_id = test_peer_id(i as u8 + 1);
        validator_order.push(peer_id.clone());
        validators.insert(
            peer_id.clone(),
            Validator {
                public_key: peer_id,
                stake: 1,
                network_address: SocketAddr::from(([127, 0, 0, 1], 8000 + i as u16)),
            },
        );
    }
    
    Committee {
        validators,
        validator_order,
    }
}

#[tokio::test]
async fn test_certificate_save_load_roundtrip() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let cert = Certificate {
        header: Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: [1u8; 32],
            parents: vec![],
            timestamp: 1000,
        },
        aggregated_signature: AggregatedSignature {
            signature: vec![1, 2, 3],
        },
        signers: vec![true, false, true],
    };
    
    // Save certificate
    let model = cert.to_persistence_model().unwrap();
    model.save(&datastore).await.unwrap();
    
    // Load certificate
    let digest_hex = model.digest.clone();
    let keys = [
        ("round".to_string(), "0".to_string()),
        ("digest".to_string(), digest_hex),
    ].into_iter().collect();
    
    let loaded_model = DAGCertificate::find_one(&datastore, keys).await.unwrap().unwrap();
    let loaded_cert = Certificate::from_persistence_model(&loaded_model).unwrap();
    
    assert_eq!(loaded_cert.header.round, cert.header.round);
    assert_eq!(loaded_cert.header.author, cert.header.author);
    assert_eq!(loaded_cert.signers, cert.signers);
}

#[tokio::test]
async fn test_batch_save_load_roundtrip() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let batch = Batch {
        transactions: vec![
            Transaction { data: vec![1, 2, 3], timestamp: 100 },
            Transaction { data: vec![4, 5, 6], timestamp: 200 },
        ],
        worker_id: 1,
        timestamp: 1000,
    };
    
    // Save batch
    let mut model = batch.to_persistence_model().unwrap();
    model.author = test_peer_id(1).to_base58();
    model.save(&datastore).await.unwrap();
    
    // Load batch
    let digest_hex = model.digest.clone();
    let keys = [("digest".to_string(), digest_hex)].into_iter().collect();
    
    let loaded_model = DAGBatch::find_one(&datastore, keys).await.unwrap().unwrap();
    let loaded_batch = Batch::from_persistence_model(&loaded_model).unwrap();
    
    assert_eq!(loaded_batch.transactions.len(), batch.transactions.len());
    assert_eq!(loaded_batch.worker_id, batch.worker_id);
}

#[tokio::test]
async fn test_dag_recovery_from_scratch() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    // Create and save test certificates
    let cert1 = Certificate {
        header: Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: [1u8; 32],
            parents: vec![],
            timestamp: 1000,
        },
        aggregated_signature: AggregatedSignature { signature: vec![1] },
        signers: vec![true],
    };
    
    let cert2 = Certificate {
        header: Header {
            author: test_peer_id(2),
            round: 1,
            batch_digest: [2u8; 32],
            parents: vec![cert1.digest()],
            timestamp: 2000,
        },
        aggregated_signature: AggregatedSignature { signature: vec![2] },
        signers: vec![true],
    };
    
    cert1.to_persistence_model().unwrap().save(&datastore).await.unwrap();
    cert2.to_persistence_model().unwrap().save(&datastore).await.unwrap();
    
    // Recover DAG
    let result = recover_dag(&datastore, RecoveryStrategy::FromScratch).await.unwrap();
    
    assert_eq!(result.certificates_loaded, 2);
    assert_eq!(result.highest_round, 1);
    assert!(!result.used_checkpoint);
    
    // Verify DAG structure
    assert_eq!(result.dag.round_size(0), 1);
    assert_eq!(result.dag.round_size(1), 1);
    
    // Verify consistency
    verify_dag_consistency(&result.dag).unwrap();
}

#[tokio::test]
async fn test_checkpoint_creation_and_recovery() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let mut dag = DAG::new();
    
    // Add some certificates
    for i in 0..3 {
        let cert = Certificate {
            header: Header {
                author: test_peer_id(i + 1),
                round: 0,
                batch_digest: [i as u8; 32],
                parents: vec![],
                timestamp: 1000 + i as u64,
            },
            aggregated_signature: AggregatedSignature { signature: vec![i as u8] },
            signers: vec![true],
        };
        dag.insert(cert.clone()).unwrap();
        cert.to_persistence_model().unwrap().save(&datastore).await.unwrap();
    }
    
    // Create states
    let committee = create_test_committee(4);
    let consensus_state = ConsensusState::new();
    let reputation_state = ReputationState::new(
        committee.validator_order.clone(),
        ReputationConfig::default(),
    );
    
    // Create checkpoint
    dag.create_checkpoint(0, &consensus_state, &reputation_state, &datastore).await.unwrap();
    
    // Verify checkpoint exists
    let checkpoint = DAGState::get_latest(&datastore).await.unwrap();
    assert!(checkpoint.is_some());
    let checkpoint = checkpoint.unwrap();
    assert_eq!(checkpoint.checkpoint_round, 0);
    assert_eq!(checkpoint.certificate_count, 3);
    
    // Recover from checkpoint
    let result = recover_dag(&datastore, RecoveryStrategy::FromCheckpoint).await.unwrap();
    assert_eq!(result.certificates_loaded, 3);
    assert!(result.used_checkpoint);
    assert_eq!(result.dag.round_size(0), 3);
}

#[tokio::test]
async fn test_checkpoint_pruning() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let dag = DAG::new();
    let consensus_state = ConsensusState::new();
    let committee = create_test_committee(4);
    let reputation_state = ReputationState::new(
        committee.validator_order.clone(),
        ReputationConfig::default(),
    );
    
    // Create multiple checkpoints
    for round in 0..5 {
        dag.create_checkpoint(round, &consensus_state, &reputation_state, &datastore).await.unwrap();
    }
    
    // Verify 5 checkpoints exist
    let mut all_checkpoints = Vec::new();
    let iterator = datastore.iterator("/dag/checkpoints/round");
    for result in iterator {
        let (key, _) = result.unwrap();
        let key_str = String::from_utf8(key.to_vec()).unwrap();
        let parts: Vec<&str> = key_str.split('/').collect();
        if let Some(round_str) = parts.get(4) {
            let keys = [("checkpoint_round".to_string(), round_str.to_string())].into_iter().collect();
            if let Some(checkpoint) = DAGState::find_one(&datastore, keys).await.unwrap() {
                all_checkpoints.push(checkpoint);
            }
        }
    }
    assert_eq!(all_checkpoints.len(), 5);
    
    // Prune, keeping only 2
    DAGState::prune_old(&datastore, 2).await.unwrap();
    
    // Verify only 2 remain
    let mut remaining = Vec::new();
    let iterator = datastore.iterator("/dag/checkpoints/round");
    for result in iterator {
        let (key, _) = result.unwrap();
        let key_str = String::from_utf8(key.to_vec()).unwrap();
        let parts: Vec<&str> = key_str.split('/').collect();
        if let Some(round_str) = parts.get(4) {
            let keys = [("checkpoint_round".to_string(), round_str.to_string())].into_iter().collect();
            if let Some(checkpoint) = DAGState::find_one(&datastore, keys).await.unwrap() {
                remaining.push(checkpoint);
            }
        }
    }
    assert_eq!(remaining.len(), 2);
    
    // Verify they are the latest ones
    let latest = DAGState::get_latest(&datastore).await.unwrap().unwrap();
    assert_eq!(latest.checkpoint_round, 4);
}

#[tokio::test]
async fn test_hybrid_recovery_strategy() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    // Test with no checkpoint (should fall back to from scratch)
    let cert = Certificate {
        header: Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: [1u8; 32],
            parents: vec![],
            timestamp: 1000,
        },
        aggregated_signature: AggregatedSignature { signature: vec![1] },
        signers: vec![true],
    };
    cert.to_persistence_model().unwrap().save(&datastore).await.unwrap();
    
    let result = recover_dag(&datastore, RecoveryStrategy::Hybrid).await.unwrap();
    assert_eq!(result.certificates_loaded, 1);
    // When no checkpoint exists, hybrid falls back to from_scratch
}

#[tokio::test]
async fn test_consensus_metadata() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    // Get or create metadata
    let mut metadata = ConsensusMetadata::get_current(&datastore).await.unwrap();
    assert_eq!(metadata.current_round, 0);
    assert_eq!(metadata.id, "current");
    
    // Update metadata
    metadata.current_round = 10;
    metadata.total_certificates = 100;
    metadata.validator_peer_id = test_peer_id(1).to_base58();
    metadata.save(&datastore).await.unwrap();
    
    // Reload and verify
    let reloaded = ConsensusMetadata::get_current(&datastore).await.unwrap();
    assert_eq!(reloaded.current_round, 10);
    assert_eq!(reloaded.total_certificates, 100);
}

#[tokio::test]
async fn test_persistence_during_multi_round_consensus() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let mut dag = DAG::new();
    
    // Round 0: genesis certificates
    let mut genesis_certs = Vec::new();
    for i in 0..4 {
        let cert = Certificate {
            header: Header {
                author: test_peer_id(i + 1),
                round: 0,
                batch_digest: [i as u8; 32],
                parents: vec![],
                timestamp: 1000 + i as u64,
            },
            aggregated_signature: AggregatedSignature { signature: vec![i as u8] },
            signers: vec![true, true, true, true],
        };
        let digest = cert.digest();
        dag.insert(cert.clone()).unwrap();
        dag.persist_certificate(&cert, &datastore).await.unwrap();
        genesis_certs.push(digest);
    }
    
    // Round 1: certificates with parents
    for i in 0..4 {
        let cert = Certificate {
            header: Header {
                author: test_peer_id(i + 1),
                round: 1,
                batch_digest: [10 + i as u8; 32],
                parents: genesis_certs.clone(),
                timestamp: 2000 + i as u64,
            },
            aggregated_signature: AggregatedSignature { signature: vec![10 + i as u8] },
            signers: vec![true, true, true, false],
        };
        dag.insert(cert.clone()).unwrap();
        dag.persist_certificate(&cert, &datastore).await.unwrap();
    }
    
    // Recover and verify
    let result = recover_dag(&datastore, RecoveryStrategy::FromScratch).await.unwrap();
    assert_eq!(result.certificates_loaded, 8); // 4 genesis + 4 round 1
    assert_eq!(result.highest_round, 1);
    assert_eq!(result.dag.round_size(0), 4);
    assert_eq!(result.dag.round_size(1), 4);
    
    verify_dag_consistency(&result.dag).unwrap();
}

#[tokio::test]
async fn test_mark_certificate_committed() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let cert = Certificate {
        header: Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: [1u8; 32],
            parents: vec![],
            timestamp: 1000,
        },
        aggregated_signature: AggregatedSignature { signature: vec![1] },
        signers: vec![true],
    };
    
    // Save certificate
    let mut model = cert.to_persistence_model().unwrap();
    assert!(!model.committed);
    model.save(&datastore).await.unwrap();
    
    // Mark as committed
    model.mark_committed(&datastore, 0).await.unwrap();
    
    // Reload and verify
    let digest_hex = model.digest.clone();
    let keys = [
        ("round".to_string(), "0".to_string()),
        ("digest".to_string(), digest_hex),
    ].into_iter().collect();
    
    let reloaded = DAGCertificate::find_one(&datastore, keys).await.unwrap().unwrap();
    assert!(reloaded.committed);
    assert_eq!(reloaded.committed_at_round, Some(0));
}

#[tokio::test]
async fn test_batch_persistence_with_certificate_reference() {
    let (datastore, _temp) = setup_test_datastore().await;
    
    let batch = Batch {
        transactions: vec![Transaction { data: vec![1, 2, 3], timestamp: 100 }],
        worker_id: 0,
        timestamp: 1000,
    };
    
    let cert = Certificate {
        header: Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: batch.digest(),
            parents: vec![],
            timestamp: 1000,
        },
        aggregated_signature: AggregatedSignature { signature: vec![1] },
        signers: vec![true],
    };
    
    let dag = DAG::new();
    
    // Persist batch with certificate reference
    let cert_digest = cert.digest();
    dag.persist_batch(&batch, &test_peer_id(1), Some(&cert_digest), &datastore).await.unwrap();
    
    // Load and verify
    let batch_digest_hex = hex::encode(batch.digest());
    let keys = [("digest".to_string(), batch_digest_hex)].into_iter().collect();
    let loaded = DAGBatch::find_one(&datastore, keys).await.unwrap().unwrap();
    
    assert!(loaded.referenced_by_cert.is_some());
    assert_eq!(loaded.transaction_count, 1);
}

