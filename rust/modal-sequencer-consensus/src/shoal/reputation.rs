use crate::narwhal::{Committee, PublicKey};
use crate::shoal::{PerformanceRecord, ReputationConfig, ReputationState};
use anyhow::Result;

/// Manager for leader reputation and selection
#[derive(Clone)]
pub struct ReputationManager {
    state: ReputationState,
    committee: Committee,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new(committee: Committee, config: ReputationConfig) -> Self {
        let validators: Vec<PublicKey> = committee.validator_order.clone();
        let state = ReputationState::new(validators, config);
        
        Self {
            state,
            committee,
        }
    }
    
    /// Get a reference to the reputation state
    pub fn get_state(&self) -> &ReputationState {
        &self.state
    }

    /// Select the leader for a given round based on reputation
    pub fn select_leader(&self, round: u64) -> PublicKey {
        // Get all validators sorted by reputation score (descending)
        let mut validators: Vec<(PublicKey, f64)> = self.state.scores
            .iter()
            .map(|(key, &score)| (key.clone(), score))
            .collect();
        
        validators.sort_by(|a, b| {
            // Sort by score descending, then by deterministic tie-breaking
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| self.deterministic_tie_break(round, &a.0, &b.0))
        });

        // Return top validator
        validators
            .first()
            .map(|(key, _)| key.clone())
            .unwrap_or_else(|| self.committee.validator_order[0].clone())
    }

    /// Deterministic tie-breaking for leader selection
    fn deterministic_tie_break(&self, round: u64, a: &PublicKey, b: &PublicKey) -> std::cmp::Ordering {
        // Hash(round || validator_key) to get deterministic but pseudorandom ordering
        use sha2::{Digest, Sha256};
        
        let mut hasher_a = Sha256::new();
        hasher_a.update(round.to_le_bytes());
        hasher_a.update(a.to_bytes()); // PeerId to bytes
        let hash_a = hasher_a.finalize();
        
        let mut hasher_b = Sha256::new();
        hasher_b.update(round.to_le_bytes());
        hasher_b.update(b.to_bytes()); // PeerId to bytes
        let hash_b = hasher_b.finalize();
        
        hash_a.cmp(&hash_b)
    }

    /// Record performance for a validator in a round
    pub fn record_performance(&mut self, record: PerformanceRecord) {
        self.state.record_performance(record);
    }

    /// Update all reputation scores based on recent performance
    pub fn update_scores(&mut self) {
        self.state.update_scores();
    }

    /// Get the reputation score for a validator
    pub fn get_score(&self, validator: &PublicKey) -> f64 {
        self.state.get_score(validator)
    }

    /// Get all reputation scores
    pub fn get_all_scores(&self) -> Vec<(PublicKey, f64)> {
        self.state.scores
            .iter()
            .map(|(key, &score)| (key.clone(), score))
            .collect()
    }

    /// Get the fallback leader if the primary leader is unavailable
    pub fn select_fallback_leader(&self, round: u64, exclude: &[PublicKey]) -> Option<PublicKey> {
        // Get validators sorted by reputation
        let mut validators: Vec<(PublicKey, f64)> = self.state.scores
            .iter()
            .filter(|(key, _)| !exclude.contains(key))
            .map(|(key, &score)| (key.clone(), score))
            .collect();
        
        validators.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| self.deterministic_tie_break(round, &a.0, &b.0))
        });

        validators.first().map(|(key, _)| key.clone())
    }
}

#[cfg(test)]
mod tests {

    /// Helper to create a deterministic PeerId for testing
    fn test_peer_id(seed: u8) -> libp2p_identity::PeerId {
        use libp2p_identity::ed25519;
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes).expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        libp2p_identity::PeerId::from_public_key(&keypair.public().into())
    }
    use super::*;
    use crate::narwhal::Validator;
    use std::net::SocketAddr;

    fn make_test_committee() -> Committee {
        let validators = vec![
            Validator {
                public_key: test_peer_id(1),
                stake: 1,
                network_address: "127.0.0.1:8000".parse::<SocketAddr>().unwrap(),
            },
            Validator {
                public_key: test_peer_id(2),
                stake: 1,
                network_address: "127.0.0.1:8001".parse::<SocketAddr>().unwrap(),
            },
            Validator {
                public_key: test_peer_id(3),
                stake: 1,
                network_address: "127.0.0.1:8002".parse::<SocketAddr>().unwrap(),
            },
            Validator {
                public_key: test_peer_id(4),
                stake: 1,
                network_address: "127.0.0.1:8003".parse::<SocketAddr>().unwrap(),
            },
        ];
        Committee::new(validators)
    }

    #[test]
    fn test_reputation_manager_initial_leader() {
        let committee = make_test_committee();
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(committee, config);

        // All validators start with same reputation, so leader should be deterministic
        let leader1 = manager.select_leader(0);
        let leader2 = manager.select_leader(0);
        assert_eq!(leader1, leader2);
    }

    #[test]
    fn test_reputation_manager_record_performance() {
        let committee = make_test_committee();
        let config = ReputationConfig::default();
        let mut manager = ReputationManager::new(committee, config);

        manager.record_performance(PerformanceRecord {
            validator: test_peer_id(1),
            round: 0,
            latency_ms: 100,
            success: true,
            timestamp: 1000,
        });

        // Scores not updated yet
        assert_eq!(manager.get_score(&vec![1]), 1.0);
        
        // Update scores
        manager.update_scores();
        
        // Score should still be good (fast and successful)
        let score = manager.get_score(&vec![1]);
        assert!(score >= 0.9); // High score for good performance
    }

    #[test]
    fn test_reputation_manager_poor_performance() {
        let committee = make_test_committee();
        let config = ReputationConfig {
            target_latency_ms: 500,
            decay_factor: 0.5, // Quick decay for testing
            ..Default::default()
        };
        let mut manager = ReputationManager::new(committee, config);

        // Record slow performance
        manager.record_performance(PerformanceRecord {
            validator: test_peer_id(1),
            round: 0,
            latency_ms: 2000, // Slow
            success: true,
            timestamp: 1000,
        });

        manager.update_scores();
        
        let score = manager.get_score(&vec![1]);
        assert!(score < 1.0); // Should decrease from perfect 1.0
    }

    #[test]
    fn test_reputation_manager_fallback_leader() {
        let committee = make_test_committee();
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(committee, config);

        let primary_leader = manager.select_leader(0);
        let fallback = manager.select_fallback_leader(0, &[primary_leader.clone()]);
        
        assert!(fallback.is_some());
        assert_ne!(fallback.unwrap(), primary_leader);
    }

    #[test]
    fn test_reputation_manager_deterministic_tie_break() {
        let committee = make_test_committee();
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(committee, config);

        // Same round should give same result
        let leader1 = manager.select_leader(5);
        let leader2 = manager.select_leader(5);
        assert_eq!(leader1, leader2);

        // Different rounds might give different results (deterministic but varies)
        let leader_r5 = manager.select_leader(5);
        let leader_r10 = manager.select_leader(10);
        // They might be the same or different, but should be deterministic
        assert_eq!(manager.select_leader(5), leader_r5);
        assert_eq!(manager.select_leader(10), leader_r10);
    }
}

