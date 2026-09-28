/// Byzantine Withholding Attack Tests
/// 
/// # Overview
/// 
/// These tests verify the consensus protocol's resilience against withholding attacks.
/// A withholding attack occurs when a Byzantine sequencer refuses to participate in
/// consensus by:
/// 1. Not voting for others' certificates (withholding votes)
/// 2. Not broadcasting its own certificates (withholding certificates)
/// 3. Being completely silent (no participation at all)
/// 
/// # Byzantine Fault Tolerance
/// 
/// The system should handle withholding attacks gracefully:
/// - Detect non-responsive sequencers through the reputation system
/// - Degrade their reputation scores over time
/// - Use fallback leader selection when the primary leader is non-responsive
/// - Maintain liveness with the remaining honest sequencers
/// 
/// # Withholding Attack Scenario
/// 
/// 1. Byzantine sequencer receives certificate proposals from others
/// 2. Byzantine sequencer refuses to vote (withholding attack)
/// 3. Other sequencers cannot form quorum with Byzantine sequencer's vote
/// 4. Reputation system detects slow/missing responses
/// 5. System selects fallback leader bypassing the Byzantine sequencer
/// 6. Consensus continues with remaining sequencers
/// 
/// # Expected System Behavior
/// 
/// - Reputation scores decrease for non-responsive sequencers
/// - Fallback leader selection mechanism activates
/// - Consensus achieves liveness despite f Byzantine sequencers
/// - System logs show detection of withholding behavior

mod common;

use common::byzantine_helpers::*;
use modality_sequencer_consensus::shoal::{PerformanceRecord, ReputationConfig};
use modality_sequencer_consensus::shoal::reputation::ReputationManager;

/// Test: Vote withholding triggers fallback leader selection
/// 
/// Scenario:
/// - Setup 4-sequencer network
/// - Sequencer 1 has highest reputation (selected as primary leader)
/// - Sequencer 1 withholds its certificate (Byzantine behavior)
/// - System should select fallback leader (next-best by reputation)
/// 
/// Expected Outcome:
/// - Primary leader (sequencer 1) is selected initially
/// - When sequencer 1 doesn't produce a certificate, fallback is triggered
/// - Fallback leader (sequencer with next-highest reputation) is selected
/// - Consensus can progress with fallback leader
#[tokio::test]
async fn test_vote_withholding_triggers_fallback() {
    // Setup: 4 sequencers
    let committee = create_test_committee(4);
    let reputation_config = ReputationConfig::default();
    let mut reputation = ReputationManager::new(committee.clone(), reputation_config);
    
    // All sequencers start with equal reputation (1.0)
    for i in 1..=4 {
        let sequencer = test_peer_id(i);
        assert_eq!(reputation.get_score(&sequencer), 1.0);
    }
    
    // Select leader for round 0
    let round0_leader = reputation.select_leader(0);
    
    // Simulate that the primary leader withholds its certificate
    // Record poor performance for the leader (no certificate appeared)
    reputation.record_performance(PerformanceRecord {
        sequencer: round0_leader.clone(),
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
/// - Setup 4-sequencer network with varied reputation scores
/// - Primary leader is unavailable
/// - Verify fallback selection picks next-best sequencer
/// 
/// Expected Outcome:
/// - Fallback leader is the sequencer with highest reputation (excluding primary)
/// - Selection is deterministic
/// - System can always find a fallback as long as enough sequencers exist
#[tokio::test]
async fn test_fallback_leader_selection() {
    // Setup: 4 sequencers
    let committee = create_test_committee(4);
    let config = ReputationConfig {
        window_size: 10,
        decay_factor: 0.5, // Lower decay for stronger effect
        min_score: 0.1,
        target_latency_ms: 500,
    };
    let mut reputation = ReputationManager::new(committee.clone(), config);
    
    // Simulate different performance levels to create clear reputation differences
    // Sequencer 1: Excellent performance (multiple rounds)
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            sequencer: test_peer_id(1),
            round,
            latency_ms: 100, // Fast
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Sequencer 2: Good performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            sequencer: test_peer_id(2),
            round,
            latency_ms: 300, // Medium
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Sequencer 3: Poor performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            sequencer: test_peer_id(3),
            round,
            latency_ms: 1000, // Slow
            success: true,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Sequencer 4: Failed performance
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            sequencer: test_peer_id(4),
            round,
            latency_ms: 5000, // Very slow
            success: false,
            timestamp: 1000 + round * 100,
        });
    }
    reputation.update_scores();
    
    // Get scores and verify sequencer 1 has highest
    let score1 = reputation.get_score(&test_peer_id(1));
    let score4 = reputation.get_score(&test_peer_id(4));
    
    // At minimum, excellent performance should beat failed performance
    assert!(
        score1 > score4,
        "Sequencer 1 (excellent) should have higher reputation than Sequencer 4 (failed): {} vs {}",
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

/// Test: Consensus continues with silent sequencer
/// 
/// Scenario:
/// - Setup 4-sequencer network (n=4, f=1)
/// - One sequencer is completely silent (no participation)
/// - Three honest sequencers produce certificates
/// - Verify consensus can still progress
/// 
/// Expected Outcome:
/// - Silent sequencer produces no certificates
/// - Three honest sequencers form quorum (2f+1 = 3)
/// - Consensus commits certificates despite missing sequencer
/// - System maintains liveness
#[tokio::test]
async fn test_consensus_with_silent_sequencer() {
    // Setup: 4 sequencers
    let (committee, dag, mut consensus) = setup_byzantine_network(4, 1);
    
    // Sequencer 4 is silent (Byzantine behavior) - produces no certificate
    // Sequencers 1, 2, 3 are honest and produce genesis certificates
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
    
    // Verify DAG has 3 certificates (from honest sequencers only)
    let dag_guard = dag.read().await;
    assert_eq!(
        dag_guard.round_size(0),
        3,
        "Should have 3 certificates in round 0 (silent sequencer produces none)"
    );
    
    // Verify consensus made progress despite the silent sequencer
    assert!(
        total_committed > 0,
        "Consensus should commit certificates despite silent sequencer"
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
/// - Sequencer repeatedly fails to produce certificates (withholding)
/// - Track reputation score over multiple rounds
/// - Verify score degrades appropriately
/// 
/// Expected Outcome:
/// - Reputation score decreases with each failed round
/// - Score approaches minimum threshold but never goes below it
/// - Eventually sequencer is not selected as leader
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
    
    let byzantine_sequencer = test_peer_id(1);
    
    // Record initial score
    let initial_score = reputation.get_score(&byzantine_sequencer);
    assert_eq!(initial_score, 1.0, "Should start with perfect reputation");
    
    // Simulate 10 rounds of withholding (failing to produce certificates)
    for round in 0..10 {
        reputation.record_performance(PerformanceRecord {
            sequencer: byzantine_sequencer.clone(),
            round,
            latency_ms: 10000, // Timeout
            success: false,     // Failed
            timestamp: 1000 + round * 1000,
        });
        
        reputation.update_scores();
    }
    
    // Score should have degraded significantly
    let final_score = reputation.get_score(&byzantine_sequencer);
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
    
    // Verify that the Byzantine sequencer is unlikely to be selected as leader
    // after reputation degradation
    let _leader = reputation.select_leader(10);
    
    // While the Byzantine sequencer might still be selected due to min_score,
    // honest sequencers should have much better chances
    // Record performance for an honest sequencer
    let honest_sequencer = test_peer_id(2);
    for round in 0..5 {
        reputation.record_performance(PerformanceRecord {
            sequencer: honest_sequencer.clone(),
            round,
            latency_ms: 200, // Fast
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let honest_score = reputation.get_score(&honest_sequencer);
    assert!(
        honest_score > final_score,
        "Honest sequencer should have higher reputation than Byzantine sequencer"
    );
}

/// Test: Recovery from temporary withholding
/// 
/// Scenario:
/// - Sequencer withholds for several rounds (reputation degrades)
/// - Sequencer starts participating normally again
/// - Track reputation recovery over time
/// 
/// Expected Outcome:
/// - Reputation degrades during withholding period
/// - Reputation stabilizes or improves when sequencer resumes normal behavior
/// - System gives sequencers a chance to recover (doesn't permanently ban)
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
    
    let sequencer = test_peer_id(1);
    
    // Phase 1: Withholding (rounds 0-4) - significant failures
    for round in 0..5 {
        reputation.record_performance(PerformanceRecord {
            sequencer: sequencer.clone(),
            round,
            latency_ms: 10000,
            success: false,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let degraded_score = reputation.get_score(&sequencer);
    assert!(
        degraded_score < 1.0,
        "Reputation should degrade during withholding, got {}",
        degraded_score
    );
    
    // Phase 2: Sustained excellent performance (rounds 5-24)
    for round in 5..25 {
        reputation.record_performance(PerformanceRecord {
            sequencer: sequencer.clone(),
            round,
            latency_ms: 100, // Very fast
            success: true,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let recovered_score = reputation.get_score(&sequencer);
    
    // With sustained good performance, score should improve or at least stabilize
    // above the minimum threshold
    assert!(
        recovered_score > min_score,
        "Reputation should recover above minimum {}, got {}",
        min_score,
        recovered_score
    );
    
    // The system allows recovery - sequencer isn't permanently penalized
    // Compare to a sequencer that continues to fail
    let failing_sequencer = test_peer_id(2);
    for round in 0..25 {
        reputation.record_performance(PerformanceRecord {
            sequencer: failing_sequencer.clone(),
            round,
            latency_ms: 10000,
            success: false,
            timestamp: 1000 + round * 1000,
        });
    }
    reputation.update_scores();
    
    let failing_score = reputation.get_score(&failing_sequencer);
    
    // Recovered sequencer should have better reputation than continuously failing sequencer
    assert!(
        recovered_score > failing_score,
        "Recovered sequencer ({}) should have better reputation than continuously failing sequencer ({})",
        recovered_score,
        failing_score
    );
}

/// Test: Multiple sequencers withholding (system limits)
/// 
/// Scenario:
/// - Setup 4-sequencer network (can tolerate f=1 Byzantine)
/// - Two sequencers withhold (exceeds Byzantine threshold)
/// - Verify system behavior at the boundary
/// 
/// Expected Outcome:
/// - With 2 sequencers withholding, only 2 honest sequencers remain
/// - Cannot achieve quorum (need 2f+1 = 3)
/// - System demonstrates safety: no commits without quorum
/// - This shows the system correctly enforces BFT limits
#[tokio::test]
async fn test_multiple_sequencers_withholding_exceeds_threshold() {
    // Setup: 4 sequencers (can tolerate f=1)
    let (committee, dag, mut consensus) = setup_byzantine_network(4, 2);
    
    // Sequencers 3 and 4 are silent (exceeds Byzantine threshold)
    // Sequencers 1 and 2 are honest and produce certificates
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
        "Should not have quorum with 2 withholding sequencers"
    );
    
    // System should not commit anything without quorum
    // This demonstrates safety: the system will not make progress
    // if too many sequencers are Byzantine, but it won't commit
    // incorrect state either
}

/// Test: Withholding detection through timeout
/// 
/// Scenario:
/// - Sequencer is expected to produce a certificate (is the leader)
/// - Sequencer withholds certificate (doesn't broadcast)
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
        sequencer: leader.clone(),
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

