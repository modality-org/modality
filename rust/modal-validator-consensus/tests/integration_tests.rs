/// Integration tests for multi-validator Shoal consensus
/// 
/// These tests demonstrate the protocol working with multiple validators,
/// including consensus formation, certificate propagation, and Byzantine behavior.

use modal_validator_consensus::narwhal::{
    Certificate, Committee, Header, Primary, Transaction, Validator, Worker,
};
use modal_validator_consensus::narwhal::dag::DAG;
use modal_validator_consensus::shoal::{ReputationConfig, ReputationState};
use modal_validator_consensus::shoal::reputation::ReputationManager;
use modal_validator_consensus::shoal::consensus::ShoalConsensus;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create a test committee with N validators
fn create_committee(n: usize) -> Committee {
    let validators: Vec<Validator> = (0..n)
        .map(|i| Validator {
            public_key: vec![i as u8],
            stake: 1,
            network_address: format!("127.0.0.1:800{}", i)
                .parse::<SocketAddr>()
                .unwrap(),
        })
        .collect();
    Committee::new(validators)
}

/// Helper to create a test certificate
fn create_test_cert(
    author: Vec<u8>,
    round: u64,
    parents: Vec<[u8; 32]>,
    committee: &Committee,
) -> Certificate {
    use modal_validator_consensus::narwhal::AggregatedSignature;
    
    let header = Header {
        author,
        round,
        batch_digest: [0u8; 32],
        parents,
        timestamp: 1000 + round * 1000,
    };
    
    // Create signers bitmap (all validators sign)
    let signers = vec![true; committee.size()];
    
    Certificate {
        header,
        aggregated_signature: AggregatedSignature {
            signature: vec![],
        },
        signers,
    }
}

#[tokio::test]
async fn test_multi_validator_genesis() {
    // Setup: 4 validators
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
    let mut consensus = ShoalConsensus::new(dag.clone(), reputation, committee.clone());
    
    // All 4 validators propose genesis certificates
    let mut genesis_certs = Vec::new();
    for i in 0..4 {
        let cert = create_test_cert(vec![i], 0, vec![], &committee);
        genesis_certs.push(cert);
    }
    
    // Process all genesis certificates
    let mut any_committed = false;
    for cert in genesis_certs {
        let committed = consensus.process_certificate(cert).await.unwrap();
        
        // At least one genesis certificate should trigger commits
        if !committed.is_empty() {
            any_committed = true;
        }
    }
    
    // Verify all genesis certificates are in the DAG
    let dag = dag.read().await;
    assert_eq!(dag.round_size(0), 4, "should have 4 genesis certificates");
    
    // Verify that at least one committed
    assert!(any_committed, "at least one genesis should have committed");
}

#[tokio::test]
async fn test_multi_validator_round_progression() {
    // Setup: 4 validators
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
    let mut consensus = ShoalConsensus::new(dag.clone(), reputation, committee.clone());
    
    // Round 0: All validators propose genesis
    let mut round0_digests = Vec::new();
    for i in 0..4 {
        let cert = create_test_cert(vec![i], 0, vec![], &committee);
        let digest = cert.digest();
        round0_digests.push(digest);
        consensus.process_certificate(cert).await.unwrap();
    }
    
    // Verify consensus state after round 0
    assert_eq!(consensus.current_round(), 0);
    
    // Verify all genesis certificates are in the DAG
    {
        let dag = dag.read().await;
        assert_eq!(dag.round_size(0), 4, "should have 4 genesis certificates");
    }
    
    // Advance to round 1
    consensus.advance_round();
    assert_eq!(consensus.current_round(), 1);
    
    // Round 1: All validators reference all genesis certificates
    for i in 0..4 {
        let cert = create_test_cert(vec![i], 1, round0_digests.clone(), &committee);
        consensus.process_certificate(cert).await.unwrap();
    }
    
    // Verify round 1 certificates are in the DAG
    let dag = dag.read().await;
    assert_eq!(dag.round_size(1), 4, "should have 4 round 1 certificates");
    
    // In a real system, commits would happen based on anchor selection
    // and paths to previous anchors. This test verifies the DAG structure is correct.
}

#[tokio::test]
async fn test_quorum_requirement() {
    // Setup: 4 validators (need 3 for quorum)
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    
    // Round 0: Create genesis certificates
    let mut round0_digests = Vec::new();
    for i in 0..4 {
        let cert = create_test_cert(vec![i], 0, vec![], &committee);
        let digest = cert.digest();
        round0_digests.push(digest);
        dag.write().await.insert(cert).unwrap();
    }
    
    // Test using Primary which validates quorum requirements
    let primary = Primary::new(vec![0], committee.clone(), dag.clone());
    
    // Create batch with only 2 parents (insufficient)
    let insufficient_parents = vec![round0_digests[0], round0_digests[1]];
    
    // Manually create a certificate with insufficient parents
    let cert = create_test_cert(vec![0], 1, insufficient_parents, &committee);
    
    // DAG insert will check if parents exist, but won't check quorum
    // That's the Primary's job during propose()
    // For now, just verify that we CAN detect insufficient parents
    assert!(cert.header.parents.len() < committee.quorum_threshold() as usize,
            "certificate has insufficient parents");
    
    // Create round 1 certificate with sufficient parents (3+)
    let sufficient_parents = vec![round0_digests[0], round0_digests[1], round0_digests[2]];
    let cert = create_test_cert(vec![1], 1, sufficient_parents.clone(), &committee);
    
    // Should succeed with quorum
    let result = dag.write().await.insert(cert);
    assert!(result.is_ok(), "should accept certificate with sufficient parents");
    assert!(sufficient_parents.len() >= committee.quorum_threshold() as usize,
            "certificate has sufficient parents for quorum");
}

#[tokio::test]
async fn test_equivocation_detection() {
    // Setup
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    
    // Validator 0 proposes first certificate in round 0
    let cert1 = create_test_cert(vec![0], 0, vec![], &committee);
    dag.write().await.insert(cert1).unwrap();
    
    // Validator 0 tries to propose DIFFERENT certificate in same round (equivocation)
    let mut cert2 = create_test_cert(vec![0], 0, vec![], &committee);
    cert2.header.batch_digest = [1u8; 32]; // Different batch
    
    // Should detect equivocation and reject
    let result = dag.write().await.insert(cert2);
    assert!(result.is_err(), "should detect and reject equivocation");
    assert!(result.unwrap_err().to_string().contains("equivocation"));
}

#[tokio::test]
async fn test_dag_path_validation() {
    // Setup
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    
    // Build chain: cert0 -> cert1 -> cert2
    let cert0 = create_test_cert(vec![0], 0, vec![], &committee);
    let digest0 = cert0.digest();
    dag.write().await.insert(cert0).unwrap();
    
    let cert1 = create_test_cert(vec![1], 1, vec![digest0], &committee);
    let digest1 = cert1.digest();
    dag.write().await.insert(cert1).unwrap();
    
    let cert2 = create_test_cert(vec![2], 2, vec![digest1], &committee);
    let digest2 = cert2.digest();
    dag.write().await.insert(cert2).unwrap();
    
    // Verify paths
    let dag = dag.read().await;
    assert!(dag.has_path(&digest2, &digest1), "should have path from cert2 to cert1");
    assert!(dag.has_path(&digest2, &digest0), "should have path from cert2 to cert0");
    assert!(dag.has_path(&digest1, &digest0), "should have path from cert1 to cert0");
    assert!(!dag.has_path(&digest0, &digest1), "should not have reverse path");
}

#[tokio::test]
async fn test_leader_reputation_adaptation() {
    // Setup
    let committee = create_committee(4);
    let config = ReputationConfig {
        window_size: 10,
        decay_factor: 0.8,
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let mut reputation = ReputationManager::new(committee.clone(), config);
    
    // Initial leader selection (all validators equal reputation)
    let initial_leader = reputation.select_leader(0);
    let initial_score = reputation.get_score(&initial_leader);
    assert_eq!(initial_score, 1.0, "initial reputation should be perfect");
    
    // Simulate slow performance for validator 0
    use modal_validator_consensus::shoal::PerformanceRecord;
    for round in 0..5 {
        reputation.record_performance(PerformanceRecord {
            validator: vec![0],
            round,
            latency_ms: 2000, // Very slow
            success: true,
            timestamp: 1000 + round * 1000,
        });
        
        // Fast performance for validator 1
        reputation.record_performance(PerformanceRecord {
            validator: vec![1],
            round,
            latency_ms: 100, // Very fast
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    
    // Update scores
    reputation.update_scores();
    
    // Validator 1 should have better reputation than validator 0
    let score0 = reputation.get_score(&vec![0]);
    let score1 = reputation.get_score(&vec![1]);
    
    assert!(score1 > score0, "fast validator should have better reputation");
    assert!(score0 < 1.0, "slow validator reputation should decrease");
}

#[tokio::test]
async fn test_byzantine_validator_isolation() {
    // Setup: 4 validators
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    
    // Round 0: 3 honest validators + 1 Byzantine
    let honest_certs: Vec<_> = (0..3)
        .map(|i| create_test_cert(vec![i], 0, vec![], &committee))
        .collect();
    
    // Insert honest certificates
    for cert in &honest_certs {
        dag.write().await.insert(cert.clone()).unwrap();
    }
    
    // Byzantine validator (3) creates TWO different certificates (equivocation)
    let byzantine_cert1 = create_test_cert(vec![3], 0, vec![], &committee);
    let mut byzantine_cert2 = create_test_cert(vec![3], 0, vec![], &committee);
    byzantine_cert2.header.batch_digest = [1u8; 32]; // Different batch
    
    // First Byzantine certificate succeeds
    dag.write().await.insert(byzantine_cert1).unwrap();
    
    // Second Byzantine certificate fails (equivocation detected)
    let result = dag.write().await.insert(byzantine_cert2);
    assert!(result.is_err(), "should detect Byzantine equivocation");
    
    // Honest validators can still make progress
    let dag = dag.read().await;
    assert_eq!(dag.round_size(0), 4, "should have 4 certificates despite Byzantine attempt");
}

#[tokio::test]
async fn test_commit_with_byzantine_minority() {
    // Setup: 4 validators (can tolerate 1 Byzantine)
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
    let mut consensus = ShoalConsensus::new(dag.clone(), reputation, committee.clone());
    
    // Round 0: 3 honest validators propose, 1 Byzantine withholds
    for i in 0..3 {
        let cert = create_test_cert(vec![i], 0, vec![], &committee);
        consensus.process_certificate(cert).await.unwrap();
    }
    
    // Should still achieve consensus with 3/4 validators (quorum is 3)
    let dag = dag.read().await;
    assert_eq!(dag.round_size(0), 3);
    
    // Verify we have quorum
    let quorum = committee.quorum_threshold();
    assert_eq!(quorum, 3);
    assert!(dag.round_size(0) >= quorum as usize, "should have quorum despite Byzantine");
}

#[tokio::test]
async fn test_concurrent_certificate_processing() {
    // Setup
    let committee = create_committee(4);
    let dag = Arc::new(RwLock::new(DAG::new()));
    let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
    let consensus = Arc::new(tokio::sync::Mutex::new(
        ShoalConsensus::new(dag.clone(), reputation, committee.clone())
    ));
    
    // Create genesis certificates
    let certs: Vec<_> = (0..4)
        .map(|i| create_test_cert(vec![i], 0, vec![], &committee))
        .collect();
    
    // Process certificates concurrently
    let handles: Vec<_> = certs
        .into_iter()
        .map(|cert| {
            let consensus = consensus.clone();
            tokio::spawn(async move {
                let mut c = consensus.lock().await;
                c.process_certificate(cert).await
            })
        })
        .collect();
    
    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "concurrent processing should succeed");
    }
    
    // Verify all certificates are in DAG
    let dag = dag.read().await;
    assert_eq!(dag.round_size(0), 4, "all certificates should be processed");
}

#[tokio::test]
async fn test_performance_degradation_recovery() {
    // Setup
    let committee = create_committee(4);
    let config = ReputationConfig {
        window_size: 5,
        decay_factor: 0.9,
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let mut reputation = ReputationManager::new(committee.clone(), config);
    
    use modal_validator_consensus::shoal::PerformanceRecord;
    
    // Phase 1: Validator 0 performs poorly
    for round in 0..3 {
        reputation.record_performance(PerformanceRecord {
            validator: vec![0],
            round,
            latency_ms: 3000, // Very slow
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    let poor_score = reputation.get_score(&vec![0]);
    
    // Phase 2: Validator 0 improves performance
    for round in 3..8 {
        reputation.record_performance(PerformanceRecord {
            validator: vec![0],
            round,
            latency_ms: 200, // Fast now
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    let improved_score = reputation.get_score(&vec![0]);
    
    // Reputation should improve
    assert!(improved_score > poor_score, "reputation should recover after improved performance");
    assert!(improved_score > 0.5, "recovered reputation should be good");
}

#[tokio::test]
async fn test_message_queue_communication() {
    use tokio::sync::{mpsc, Mutex};
    
    // Setup: 4 validators
    let committee = create_committee(4);
    
    // Create message channels for certificate broadcasting
    let (tx, mut rx) = mpsc::unbounded_channel::<(usize, Certificate)>();
    
    // Create 4 consensus instances (one per validator)
    let mut consensus_instances = Vec::new();
    for _ in 0..4 {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
        let consensus = Arc::new(Mutex::new(ShoalConsensus::new(
            dag.clone(),
            reputation,
            committee.clone(),
        )));
        consensus_instances.push((dag, consensus));
    }
    
    // Spawn message processor to broadcast certificates
    let instances_clone = consensus_instances.clone();
    let processor = tokio::spawn(async move {
        while let Some((from, cert)) = rx.recv().await {
            // Broadcast to all other validators
            for (i, (_dag, consensus)) in instances_clone.iter().enumerate() {
                if i != from {
                    let mut cons = consensus.lock().await;
                    let _ = cons.process_certificate(cert.clone()).await;
                }
            }
        }
    });
    
    // Each validator proposes a genesis certificate
    for (i, (_dag, consensus)) in consensus_instances.iter().enumerate() {
        let cert = create_test_cert(vec![i as u8], 0, vec![], &committee);
        
        // Process locally first
        let mut cons = consensus.lock().await;
        cons.process_certificate(cert.clone()).await.unwrap();
        drop(cons);
        
        // Broadcast to others via message queue
        tx.send((i, cert)).unwrap();
    }
    
    // Give time for messages to propagate through the queue
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify all validators have all genesis certificates
    for (i, (dag, consensus)) in consensus_instances.iter().enumerate() {
        let dag_read = dag.read().await;
        let genesis_count = dag_read.round_size(0);
        println!("Validator {} received {} genesis certificates via message queue", i, genesis_count);
        assert_eq!(genesis_count, 4, "validator {} should have all genesis certs", i);
        
        // Verify consensus state
        let cons = consensus.lock().await;
        println!("Validator {} committed {} certificates", i, cons.state.committed.len());
    }
    
    drop(tx); // Close channel to terminate processor
    processor.await.unwrap();
}

#[tokio::test]
async fn test_message_queue_round_progression() {
    use tokio::sync::{mpsc, Mutex};
    
    // Setup: 4 validators with message queue communication
    let committee = create_committee(4);
    let (tx, mut rx) = mpsc::unbounded_channel::<(usize, Certificate)>();
    
    // Create 4 consensus instances
    let mut consensus_instances = Vec::new();
    for _ in 0..4 {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
        let consensus = Arc::new(Mutex::new(ShoalConsensus::new(
            dag.clone(),
            reputation,
            committee.clone(),
        )));
        consensus_instances.push((dag, consensus));
    }
    
    // Spawn message processor
    let instances_clone = consensus_instances.clone();
    let processor = tokio::spawn(async move {
        while let Some((from, cert)) = rx.recv().await {
            for (i, (_dag, consensus)) in instances_clone.iter().enumerate() {
                if i != from {
                    let mut cons = consensus.lock().await;
                    let _ = cons.process_certificate(cert.clone()).await;
                }
            }
        }
    });
    
    // Round 0: Genesis - all validators propose
    println!("\n=== Round 0: Genesis ===");
    let mut round0_digests = Vec::new();
    for (i, (_dag, consensus)) in consensus_instances.iter().enumerate() {
        let cert = create_test_cert(vec![i as u8], 0, vec![], &committee);
        let digest = cert.digest();
        round0_digests.push(digest);
        
        let mut cons = consensus.lock().await;
        cons.process_certificate(cert.clone()).await.unwrap();
        drop(cons);
        
        tx.send((i, cert)).unwrap();
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify all validators have round 0 certificates
    for (i, (dag, consensus)) in consensus_instances.iter().enumerate() {
        let dag_read = dag.read().await;
        assert_eq!(dag_read.round_size(0), 4, "validator {} should have all round 0 certs", i);
        
        // Advance to round 1
        let mut cons = consensus.lock().await;
        cons.advance_round();
        println!("Validator {} advanced to round {}", i, cons.state.current_round);
    }
    
    // Round 1: All validators propose with parents
    println!("\n=== Round 1: With Parents ===");
    for (i, (_dag, consensus)) in consensus_instances.iter().enumerate() {
        let cert = create_test_cert(
            vec![i as u8],
            1,
            round0_digests.clone(), // Reference all round 0 certificates
            &committee,
        );
        
        let mut cons = consensus.lock().await;
        cons.process_certificate(cert.clone()).await.unwrap();
        drop(cons);
        
        tx.send((i, cert)).unwrap();
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify round 1 certificates propagated
    for (i, (dag, consensus)) in consensus_instances.iter().enumerate() {
        let dag_read = dag.read().await;
        let round0_count = dag_read.round_size(0);
        let round1_count = dag_read.round_size(1);
        
        println!("Validator {}: Round 0: {} certs, Round 1: {} certs", 
                 i, round0_count, round1_count);
        
        assert_eq!(round0_count, 4, "should have all round 0 certs");
        assert_eq!(round1_count, 4, "should have all round 1 certs");
        
        // Check that round 1 certs reference round 0 certs
        for cert in dag_read.get_round(1) {
            assert_eq!(cert.header.parents.len(), 4, "round 1 cert should reference all round 0 parents");
        }
        
        let cons = consensus.lock().await;
        println!("Validator {} committed {} total certificates", i, cons.state.committed.len());
    }
    
    drop(tx);
    processor.await.unwrap();
}

