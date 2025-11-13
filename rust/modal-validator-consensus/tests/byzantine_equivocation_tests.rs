/// Byzantine Equivocation Detection Tests
/// 
/// # Overview
/// 
/// These tests verify the consensus protocol's ability to detect and handle equivocation
/// attacks. Equivocation occurs when a Byzantine (malicious) validator creates two
/// conflicting certificates for the same round, attempting to show different views to
/// different honest validators.
/// 
/// # Byzantine Fault Tolerance
/// 
/// In a BFT system with n validators, we can tolerate up to f Byzantine validators
/// where n = 3f + 1. For these tests, we use n=4 validators, allowing us to tolerate
/// f=1 Byzantine validator while maintaining consensus with 2f+1=3 honest validators.
/// 
/// # Equivocation Attack Scenario
/// 
/// 1. Byzantine validator creates certificate A for round R
/// 2. Byzantine validator creates certificate B for round R (conflicting with A)
/// 3. Byzantine validator sends A to some validators and B to others
/// 4. Goal: Split the honest validators' view of the DAG
/// 
/// # Expected System Behavior
/// 
/// - Honest validators detect equivocation when they see both certificates
/// - Both conflicting certificates are rejected by the DAG
/// - The Byzantine validator is prevented from participating in round R
/// - Consensus continues with the remaining 3 honest validators
/// - Safety is maintained: all honest validators agree on the same state

mod common;

use common::byzantine_helpers::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test: Single validator equivocation is detected
/// 
/// Scenario:
/// - Setup a 4-validator network
/// - One validator creates two conflicting certificates for round 0 (genesis)
/// - Insert the first certificate successfully
/// - Attempt to insert the second certificate
/// 
/// Expected Outcome:
/// - First certificate is accepted
/// - Second certificate is detected as equivocation
/// - DAG rejects the second certificate
/// - detect_equivocation() returns true for the conflicting certificate
#[tokio::test]
async fn test_equivocation_detection_single_validator() {
    // Setup: 4 validators (n=4, f=1)
    let (committee, dag, _consensus) = setup_byzantine_network(4, 1);
    
    // Byzantine validator will equivocate
    let byzantine_validator = test_peer_id(1);
    
    // Create two conflicting certificates for the same round
    let (cert1, cert2) = create_conflicting_certificates(
        byzantine_validator,
        0, // Genesis round
        vec![], // No parents for genesis
        &committee,
    );
    
    // Insert first certificate - should succeed
    {
        let mut dag_guard = dag.write().await;
        let result = dag_guard.insert(cert1.clone());
        assert!(result.is_ok(), "First certificate should be inserted successfully");
        assert_eq!(dag_guard.round_size(0), 1);
    }
    
    // Check that equivocation is detected for the second certificate
    assert_equivocation_detected(&dag, &cert2).await;
    
    // Attempt to insert second certificate - should fail
    assert_insert_fails_equivocation(&dag, cert2).await;
    
    // Verify DAG state: only one certificate from the Byzantine validator
    let dag_guard = dag.read().await;
    assert_eq!(
        dag_guard.round_size(0),
        1,
        "Only the first certificate should be in the DAG"
    );
}

/// Test: Equivocation is rejected by honest validators
/// 
/// Scenario:
/// - Setup a 4-validator network with 3 honest validators
/// - Byzantine validator attempts equivocation
/// - Verify that honest validators' DAGs reject the conflicting certificate
/// 
/// Expected Outcome:
/// - Honest validators maintain consistent state
/// - No honest validator accepts both conflicting certificates
/// - The Byzantine validator's equivocation is isolated
#[tokio::test]
async fn test_equivocation_rejected_by_honest_validators() {
    // Create 3 separate DAGs representing 3 honest validators
    let committee = create_test_committee(4);
    let honest_dag1 = Arc::new(RwLock::new(modal_validator_consensus::narwhal::dag::DAG::new()));
    let honest_dag2 = Arc::new(RwLock::new(modal_validator_consensus::narwhal::dag::DAG::new()));
    let honest_dag3 = Arc::new(RwLock::new(modal_validator_consensus::narwhal::dag::DAG::new()));
    
    // Byzantine validator creates conflicting certificates
    let byzantine_validator = test_peer_id(4);
    let (cert1, cert2) = create_conflicting_certificates(
        byzantine_validator,
        0,
        vec![],
        &committee,
    );
    
    // Scenario: Byzantine validator sends cert1 to validators 1 and 2
    {
        let mut dag1 = honest_dag1.write().await;
        assert!(dag1.insert(cert1.clone()).is_ok());
    }
    {
        let mut dag2 = honest_dag2.write().await;
        assert!(dag2.insert(cert1.clone()).is_ok());
    }
    
    // Byzantine validator sends cert2 to validator 3
    {
        let mut dag3 = honest_dag3.write().await;
        assert!(dag3.insert(cert2.clone()).is_ok());
    }
    
    // Now validators gossip certificates to each other
    // When validator 1 receives cert2, it should detect equivocation
    assert_equivocation_detected(&honest_dag1, &cert2).await;
    assert_insert_fails_equivocation(&honest_dag1, cert2.clone()).await;
    
    // When validator 3 receives cert1, it should detect equivocation
    assert_equivocation_detected(&honest_dag3, &cert1).await;
    assert_insert_fails_equivocation(&honest_dag3, cert1.clone()).await;
    
    // Verify each honest validator has exactly one certificate (no duplicates)
    {
        let dag1 = honest_dag1.read().await;
        assert_eq!(dag1.round_size(0), 1);
    }
    {
        let dag2 = honest_dag2.read().await;
        assert_eq!(dag2.round_size(0), 1);
    }
    {
        let dag3 = honest_dag3.read().await;
        assert_eq!(dag3.round_size(0), 1);
    }
}

/// Test: Consensus continues after equivocation
/// 
/// Scenario:
/// - Setup 4-validator network
/// - One validator equivocates in genesis round
/// - Three honest validators each propose valid certificates
/// - Verify consensus can still progress
/// 
/// Expected Outcome:
/// - Equivocating validator is excluded from round 0
/// - Three honest validators form a quorum (2f+1 = 3)
/// - Consensus commits certificates from honest validators
/// - System demonstrates liveness despite Byzantine behavior
#[tokio::test]
async fn test_consensus_continues_after_equivocation() {
    // Setup: 4 validators
    let (committee, dag, mut consensus) = setup_byzantine_network(4, 1);
    
    // Validator 1 equivocates in genesis round
    let byzantine_validator = test_peer_id(1);
    let (cert1, cert2) = create_conflicting_certificates(
        byzantine_validator,
        0,
        vec![],
        &committee,
    );
    
    // Insert first equivocating certificate
    {
        let mut dag_guard = dag.write().await;
        dag_guard.insert(cert1.clone()).unwrap();
    }
    
    // Attempt to insert second equivocating certificate - should fail
    assert_insert_fails_equivocation(&dag, cert2).await;
    
    // Three honest validators propose genesis certificates
    let mut honest_certs = Vec::new();
    for i in 2..=4 {
        let honest_validator = test_peer_id(i);
        let cert = create_test_certificate(
            honest_validator,
            0,
            vec![],
            [i as u8; 32], // Different batch for each
            &committee,
        );
        honest_certs.push(cert);
    }
    
    // Process honest certificates through consensus
    let mut total_committed = 0;
    for cert in honest_certs {
        let committed = consensus.process_certificate(cert).await.unwrap();
        total_committed += committed.len();
    }
    
    // Verify DAG has 4 certificates total (1 from Byzantine, 3 from honest)
    let dag_guard = dag.read().await;
    assert_eq!(
        dag_guard.round_size(0),
        4,
        "Should have 4 certificates in round 0 (1 Byzantine + 3 honest)"
    );
    
    // Verify that consensus made progress despite the Byzantine validator
    // At least some certificates should have been committed
    assert!(
        total_committed > 0,
        "Consensus should commit certificates despite equivocation"
    );
    
    // Verify consensus state
    assert_eq!(consensus.current_round(), 0);
}

/// Test: Multiple equivocations from the same validator
/// 
/// Scenario:
/// - Byzantine validator attempts multiple equivocations in sequence
/// - First conflicting pair in round 0
/// - Second conflicting pair in round 1
/// 
/// Expected Outcome:
/// - All equivocations are detected
/// - Only the first certificate of each pair is accepted
/// - Byzantine validator cannot successfully equivocate multiple times
#[tokio::test]
async fn test_multiple_equivocations_same_validator() {
    let (committee, dag, _consensus) = setup_byzantine_network(4, 1);
    let byzantine_validator = test_peer_id(1);
    
    // First equivocation in round 0
    let (cert0a, cert0b) = create_conflicting_certificates(
        byzantine_validator.clone(),
        0,
        vec![],
        &committee,
    );
    
    {
        let mut dag_guard = dag.write().await;
        assert!(dag_guard.insert(cert0a.clone()).is_ok());
    }
    assert_insert_fails_equivocation(&dag, cert0b).await;
    
    // Get the first certificate's digest to use as parent
    let cert0a_digest = cert0a.digest();
    
    // Second equivocation attempt in round 1
    let (cert1a, cert1b) = create_conflicting_certificates(
        byzantine_validator,
        1,
        vec![cert0a_digest],
        &committee,
    );
    
    {
        let mut dag_guard = dag.write().await;
        assert!(dag_guard.insert(cert1a.clone()).is_ok());
    }
    assert_insert_fails_equivocation(&dag, cert1b).await;
    
    // Verify only 2 certificates from the Byzantine validator (one per round)
    let dag_guard = dag.read().await;
    assert_eq!(dag_guard.round_size(0), 1);
    assert_eq!(dag_guard.round_size(1), 1);
}

/// Test: Equivocation with different parent sets
/// 
/// Scenario:
/// - Byzantine validator creates two certificates in round 1
/// - Both certificates reference different parent sets from round 0
/// - This is a more sophisticated equivocation attempt
/// 
/// Expected Outcome:
/// - Equivocation is still detected despite different parents
/// - DAG correctly identifies same (author, round) as equivocation
#[tokio::test]
async fn test_equivocation_different_parents() {
    let (committee, dag, _consensus) = setup_byzantine_network(4, 1);
    
    // Setup: Create multiple genesis certificates to serve as parents
    let mut parent_digests = Vec::new();
    for i in 1..=3 {
        let genesis_cert = create_test_certificate(
            test_peer_id(i),
            0,
            vec![],
            [i as u8; 32],
            &committee,
        );
        let digest = genesis_cert.digest();
        parent_digests.push(digest);
        
        let mut dag_guard = dag.write().await;
        dag_guard.insert(genesis_cert).unwrap();
    }
    
    // Byzantine validator creates two round-1 certificates with different parent sets
    let byzantine_validator = test_peer_id(4);
    
    // First certificate references first two parents
    let cert1 = create_test_certificate(
        byzantine_validator.clone(),
        1,
        vec![parent_digests[0], parent_digests[1]],
        [10u8; 32],
        &committee,
    );
    
    // Second certificate references different parents (last two)
    let cert2 = create_test_certificate(
        byzantine_validator,
        1,
        vec![parent_digests[1], parent_digests[2]],
        [20u8; 32],
        &committee,
    );
    
    // Insert first certificate
    {
        let mut dag_guard = dag.write().await;
        assert!(dag_guard.insert(cert1.clone()).is_ok());
    }
    
    // Second certificate should still be detected as equivocation
    assert_equivocation_detected(&dag, &cert2).await;
    assert_insert_fails_equivocation(&dag, cert2).await;
}

/// Test: Non-equivocating certificates from same validator in different rounds
/// 
/// Scenario:
/// - Validator creates one certificate per round (normal behavior)
/// - Verify that this is NOT detected as equivocation
/// 
/// Expected Outcome:
/// - All certificates are accepted
/// - No false positives for equivocation detection
/// - Validator can participate normally across multiple rounds
#[tokio::test]
async fn test_no_false_positive_equivocation() {
    let (committee, dag, _consensus) = setup_byzantine_network(4, 1);
    let validator = test_peer_id(1);
    
    // Round 0
    let cert0 = create_test_certificate(
        validator.clone(),
        0,
        vec![],
        [0u8; 32],
        &committee,
    );
    let digest0 = cert0.digest();
    
    {
        let mut dag_guard = dag.write().await;
        assert!(dag_guard.insert(cert0.clone()).is_ok());
    }
    
    // Round 1 - different round, so not equivocation
    let cert1 = create_test_certificate(
        validator.clone(),
        1,
        vec![digest0],
        [1u8; 32],
        &committee,
    );
    let digest1 = cert1.digest();
    
    {
        let mut dag_guard = dag.write().await;
        assert!(!dag_guard.detect_equivocation(&cert1), "Should not detect equivocation for different rounds");
        assert!(dag_guard.insert(cert1.clone()).is_ok());
    }
    
    // Round 2 - still not equivocation
    let cert2 = create_test_certificate(
        validator,
        2,
        vec![digest1],
        [2u8; 32],
        &committee,
    );
    
    {
        let mut dag_guard = dag.write().await;
        assert!(!dag_guard.detect_equivocation(&cert2), "Should not detect equivocation for different rounds");
        assert!(dag_guard.insert(cert2.clone()).is_ok());
    }
    
    // Verify all certificates were inserted
    let dag_guard = dag.read().await;
    assert_eq!(dag_guard.round_size(0), 1);
    assert_eq!(dag_guard.round_size(1), 1);
    assert_eq!(dag_guard.round_size(2), 1);
    assert_eq!(dag_guard.highest_round(), 2);
}

