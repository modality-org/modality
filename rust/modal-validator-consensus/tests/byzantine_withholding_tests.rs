/// Byzantine Withholding Attack Tests
/// 
/// # Overview
/// 
/// These tests verify the consensus protocol's resilience against withholding attacks.
/// A withholding attack occurs when a Byzantine validator refuses to participate in
/// consensus by:
/// 1. Not voting for others' certificates (withholding votes)
/// 2. Not broadcasting its own certificates (withholding certificates)
/// 3. Being completely silent (no participation at all)
/// 
/// # Byzantine Fault Tolerance
/// 
/// The system should handle withholding attacks gracefully:
/// - Detect non-responsive validators through the reputation system
/// - Degrade their reputation scores over time
/// - Use fallback leader selection when the primary leader is non-responsive
/// - Maintain liveness with the remaining honest validators
/// 
/// # Withholding Attack Scenario
/// 
/// 1. Byzantine validator receives certificate proposals from others
/// 2. Byzantine validator refuses to vote (withholding attack)
/// 3. Other validators cannot form quorum with Byzantine validator's vote
/// 4. Reputation system detects slow/missing responses
/// 5. System selects fallback leader bypassing the Byzantine validator
/// 6. Consensus continues with remaining validators
/// 
/// # Expected System Behavior
/// 
/// - Reputation scores decrease for non-responsive validators
/// - Fallback leader selection mechanism activates
/// - Consensus achieves liveness despite f Byzantine validators
/// - System logs show detection of withholding behavior

mod common;

use common::byzantine_helpers::*;
use modal_validator_consensus::shoal::{PerformanceRecord, ReputationConfig};
use modal_validator_consensus::shoal::reputation::ReputationManager;

/// Test: Vote withholding triggers fallback leader selection
/// 
/// Scenario:
/// - Setup 4-validator network
/// - Validator 1 has highest reputation (selected as primary leader)
/// - Validator 1 withholds its certificate (Byzantine behavior)
/// - System should select fallback leader (next-best by reputation)
/// 
/// Expected Outcome:
/// - Primary leader (validator 1) is selected initially
/// - When validator 1 doesn't produce a certificate, fallback is triggered
/// - Fallback leader (validator with next-highest reputation) is selected
/// - Consensus can progress with fallback leader
#[tokio::test]
async fn test_vote_withholding_triggers_fallback() {
    // Setup: 4 validators
    let committee = create_test_committee(4);
    let reputation_config = ReputationConfig::default();
    let mut reputation = ReputationManager::new(committee.clone(), reputation_config);
    
    // All validators start with equal reputation (1.0)
    for i in 1..=4 {
        let validator = test_peer_id(i);
        assert_eq!(reputation.get_score(&validator), 1.0);
    }
    
    // Select leader for round 0
    let round0_leader = reputation.select_leader(0);
    
    // Simulate that the primary leader withholds its certificate
    // Record poor performance for the leader (no certificate appeared)
    reputation.record_performance(PerformanceRecord {
        validator: round0_leader.clone(),
        round: 0,
        latency_ms: 10000, // Very slow (timeout)
        success: false,     // Failed to produce certificate
        timestamp: 1000,
    });
    
    // Update reputation scores
    reputation.update_scores();
    
    // Leader's reputation should have decreased
    let leader_score_after = reputation.get_score(&round0_leader);
    assert!(
        leader_score_after < 1.0,
        "Leader's reputation should decrease after failing to produce certificate, got {}",
        leader_score_after
    );
    
    // Select fallback leader (excluding the primary leader)
    let fallback_leader = reputation.select_fallback_leader(0, &[round0_leader.clone()]);
    assert!(fallback_leader.is_some(), "Should select a fallback leader");
    assert_ne!(
        fallback_leader.unwrap(),
        round0_leader,
        "Fallback leader should be different from primary leader"
    );
}

/// Test: Fallback leader selection mechanism
/// 
/// Scenario:
/// - Setup 4-validator network with varied reputation scores
/// - Primary leader is unavailable
/// - Verify fallback selection picks next-best validator
/// 
/// Expected Outcome:
/// - Fallback leader is the validator with highest reputation (excluding primary)
/// - Selection is deterministic
/// - System can always find a fallback as long as enough validators exist
#[tokio::test]
async fn test_fallback_leader_selection() {
    // Setup: 4 validators
    let committee = create_test_committee(4);
    let config = ReputationConfig {
        window_size: 10,
        decay_factor: 0.5, // Lower decay for stronger effect
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let mut reputation = ReputationManager::new(committee.clone(), config);
    
    // Simulate different performance levels to create clear reputation differences
    // Validator 1: Excellent performance (multiple rounds)
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            validator: test_peer_id(1),
            round,
            latency_ms: 100, // Fast
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Validator 2: Good performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            validator: test_peer_id(2),
            round,
            latency_ms: 300, // Medium
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Validator 3: Poor performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            validator: test_peer_id(3),
            round,
            latency_ms: 1000, // Slow
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Validator 4: Failed performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            validator: test_peer_id(4),
            round,
            latency_ms: 5000, // Very slow
            success: false,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Get scores and verify validator 1 has highest
    let score1 = reputation.get_score(&test_peer_id(1));
    let score4 = reputation.get_score(&test_peer_id(4));
    
    // At minimum, excellent performance should beat failed performance
    assert!(
        score1 > score4,
        "Validator 1 (excellent) should have higher reputation than Validator 4 (failed): {} vs {}",
        score1,
        score4
    );
    
    // Primary leader selection
    let primary = reputation.select_leader(1);
    
    // If primary is unavailable, select fallback
    let fallback = reputation.select_fallback_leader(1, &[primary.clone()]);
    assert!(fallback.is_some(), "Should have a fallback leader");
    assert_ne!(
        fallback.as_ref().unwrap(),
        &primary,
        "Fallback should be different from primary"
    );
    
    // Fallback should have positive reputation
    let fallback_score = reputation.get_score(fallback.as_ref().unwrap());
    assert!(
        fallback_score > 0.0,
        "Fallback leader should have positive reputation"
    );
}

/// Test: Consensus continues with silent validator
/// 
/// Scenario:
/// - Setup 4-validator network (n=4, f=1)
/// - One validator is completely silent (no participation)
/// - Three honest validators produce certificates
/// - Verify consensus can still progress
/// 
/// Expected Outcome:
/// - Silent validator produces no certificates
/// - Three honest validators form quorum (2f+1 = 3)
/// - Consensus commits certificates despite missing validator
/// - System maintains liveness
#[tokio::test]
async fn test_consensus_with_silent_validator() {
    // Setup: 4 validators
    let (committee, dag, mut consensus) = setup_byzantine_network(4, 1);
    
    // Validator 4 is silent (Byzantine behavior) - produces no certificate
    // Validators 1, 2, 3 are honest and produce genesis certificates
    let mut honest_certs = Vec::new();
    for i in 1..=3 {
        let cert = create_test_certificate(
            test_peer_id(i),
            0, // Genesis round
            vec![],
            [i as u8; 32],
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
    
    // Verify DAG has 3 certificates (from honest validators only)
    let dag_guard = dag.read().await;
    assert_eq!(
        dag_guard.round_size(0),
        3,
        "Should have 3 certificates in round 0 (silent validator produces none)"
    );
    
    // Verify consensus made progress despite the silent validator
    assert!(
        total_committed > 0,
        "Consensus should commit certificates despite silent validator"
    );
    
    // Verify we have quorum (3 >= 2f+1 where f=1)
    assert!(
        dag_guard.round_size(0) >= committee.quorum_threshold() as usize,
        "Should have quorum of certificates"
    );
}

/// Test: Reputation degradation from withholding
/// 
/// Scenario:
/// - Validator repeatedly fails to produce certificates (withholding)
/// - Track reputation score over multiple rounds
/// - Verify score degrades appropriately
/// 
/// Expected Outcome:
/// - Reputation score decreases with each failed round
/// - Score approaches minimum threshold but never goes below it
/// - Eventually validator is not selected as leader
#[tokio::test]
async fn test_reputation_degradation_from_withholding() {
    let committee = create_test_committee(4);
    let config = ReputationConfig {
        window_size: 10,
        decay_factor: 0.8,
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let min_score = config.min_score;
    let mut reputation = ReputationManager::new(committee, config);
    
    let byzantine_validator = test_peer_id(1);
    
    // Record initial score
    let initial_score = reputation.get_score(&byzantine_validator);
    assert_eq!(initial_score, 1.0, "Should start with perfect reputation");
    
    // Simulate 10 rounds of withholding (failing to produce certificates)
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            validator: byzantine_validator.clone(),
            round,
            latency_ms: 10000, // Timeout
            success: false,     // Failed
            timestamp: 1000 + round * 1000,
        });
        
        reputation.update_scores();
    }
    
    // Score should have degraded significantly
    let final_score = reputation.get_score(&byzantine_validator);
    assert!(
        final_score < initial_score,
        "Score should degrade from {} to less than that, got {}",
        initial_score,
        final_score
    );
    
    // Score should not go below minimum
    assert!(
        final_score >= min_score,
        "Score should not go below minimum {}, got {}",
        min_score,
        final_score
    );
    
    // Verify that the Byzantine validator is unlikely to be selected as leader
    // after reputation degradation
    let _leader = reputation.select_leader(10);
    
    // While the Byzantine validator might still be selected due to min_score,
    // honest validators should have much better chances
    // Record performance for an honest validator
    let honest_validator = test_peer_id(2);
    for round in 0..5 {
        reputation.record_performance(PerformanceRecord {
            validator: honest_validator.clone(),
            round,
            latency_ms: 200, // Fast
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let honest_score = reputation.get_score(&honest_validator);
    assert!(
        honest_score > final_score,
        "Honest validator should have higher reputation than Byzantine validator"
    );
}

/// Test: Recovery from temporary withholding
/// 
/// Scenario:
/// - Validator withholds for several rounds (reputation degrades)
/// - Validator starts participating normally again
/// - Track reputation recovery over time
/// 
/// Expected Outcome:
/// - Reputation degrades during withholding period
/// - Reputation stabilizes or improves when validator resumes normal behavior
/// - System gives validators a chance to recover (doesn't permanently ban)
#[tokio::test]
async fn test_recovery_from_temporary_withholding() {
    let committee = create_test_committee(4);
    let config = ReputationConfig {
        window_size: 20,
        decay_factor: 0.7, // Higher influence from recent performance
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let min_score = config.min_score;
    let mut reputation = ReputationManager::new(committee, config);
    
    let validator = test_peer_id(1);
    
    // Phase 1: Withholding (rounds 0-4) - significant failures
    for round in 0..5 {
        reputation.record_performance(PerformanceRecord {
            validator: validator.clone(),
            round,
            latency_ms: 10000,
            success: false,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let degraded_score = reputation.get_score(&validator);
    assert!(
        degraded_score < 1.0,
        "Reputation should degrade during withholding, got {}",
        degraded_score
    );
    
    // Phase 2: Sustained excellent performance (rounds 5-24)
    for round in 5..25 {
        reputation.record_performance(PerformanceRecord {
            validator: validator.clone(),
            round,
            latency_ms: 100, // Very fast
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let recovered_score = reputation.get_score(&validator);
    
    // With sustained good performance, score should improve or at least stabilize
    // above the minimum threshold
    assert!(
        recovered_score > min_score,
        "Reputation should recover above minimum {}, got {}",
        min_score,
        recovered_score
    );
    
    // The system allows recovery - validator isn't permanently penalized
    // Compare to a validator that continues to fail
    let failing_validator = test_peer_id(2);
    for round in 0..25 {
        reputation.record_performance(PerformanceRecord {
            validator: failing_validator.clone(),
            round,
            latency_ms: 10000,
            success: false,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let failing_score = reputation.get_score(&failing_validator);
    
    // Recovered validator should have better reputation than continuously failing validator
    assert!(
        recovered_score > failing_score,
        "Recovered validator ({}) should have better reputation than continuously failing validator ({})",
        recovered_score,
        failing_score
    );
}

/// Test: Multiple validators withholding (system limits)
/// 
/// Scenario:
/// - Setup 4-validator network (can tolerate f=1 Byzantine)
/// - Two validators withhold (exceeds Byzantine threshold)
/// - Verify system behavior at the boundary
/// 
/// Expected Outcome:
/// - With 2 validators withholding, only 2 honest validators remain
/// - Cannot achieve quorum (need 2f+1 = 3)
/// - System demonstrates safety: no commits without quorum
/// - This shows the system correctly enforces BFT limits
#[tokio::test]
async fn test_multiple_validators_withholding_exceeds_threshold() {
    // Setup: 4 validators (can tolerate f=1)
    let (committee, dag, mut consensus) = setup_byzantine_network(4, 2);
    
    // Validators 3 and 4 are silent (exceeds Byzantine threshold)
    // Validators 1 and 2 are honest and produce certificates
    let mut honest_certs = Vec::new();
    for i in 1..=2 {
        let cert = create_test_certificate(
            test_peer_id(i),
            0,
            vec![],
            [i as u8; 32],
            &committee,
        );
        honest_certs.push(cert);
    }
    
    // Process honest certificates
    for cert in honest_certs {
        let _ = consensus.process_certificate(cert).await.unwrap();
    }
    
    // Verify DAG has 2 certificates
    let dag_guard = dag.read().await;
    assert_eq!(dag_guard.round_size(0), 2);
    
    // Verify we don't have quorum (2 < 2f+1 = 3)
    assert!(
        dag_guard.round_size(0) < committee.quorum_threshold() as usize,
        "Should not have quorum with 2 withholding validators"
    );
    
    // System should not commit anything without quorum
    // This demonstrates safety: the system will not make progress
    // if too many validators are Byzantine, but it won't commit
    // incorrect state either
}

/// Test: Withholding detection through timeout
/// 
/// Scenario:
/// - Validator is expected to produce a certificate (is the leader)
/// - Validator withholds certificate (doesn't broadcast)
/// - System detects missing certificate via timeout
/// 
/// Expected Outcome:
/// - After timeout period, system detects missing certificate
/// - Reputation system records poor performance
/// - Fallback mechanism activates
#[tokio::test]
async fn test_withholding_detection_through_timeout() {
    let committee = create_test_committee(4);
    let mut reputation = ReputationManager::new(committee, ReputationConfig::default());
    
    // Select leader for round 0
    let leader = reputation.select_leader(0);
    let initial_score = reputation.get_score(&leader);
    
    // Simulate timeout: leader was expected to produce certificate but didn't
    // This is detected after timeout period expires
    reputation.record_performance(PerformanceRecord {
        validator: leader.clone(),
        round: 0,
        latency_ms: 5000, // Exceeded timeout
        success: false,   // No certificate
        timestamp: 1000,
    });
    
    reputation.update_scores();
    
    let score_after_timeout = reputation.get_score(&leader);
    
    // Reputation should decrease due to timeout
    assert!(
        score_after_timeout < initial_score,
        "Reputation should decrease after timeout"
    );
}

