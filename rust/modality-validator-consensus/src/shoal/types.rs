use crate::narwhal::{CertificateDigest, PublicKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Performance record for a validator in a specific round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    /// Validator that produced the certificate
    pub validator: PublicKey,
    /// Round number
    pub round: u64,
    /// Time from round start to certificate appearance (milliseconds)
    pub latency_ms: u64,
    /// Whether the certificate was formed successfully
    pub success: bool,
    /// Timestamp when this record was created
    pub timestamp: u64,
}

/// Configuration for the reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Number of recent performance records to keep (sliding window)
    pub window_size: usize,
    /// Decay factor for old observations (0.0 to 1.0)
    /// Higher = more weight on recent performance
    pub decay_factor: f64,
    /// Minimum reputation score (prevents complete exclusion)
    pub min_score: f64,
    /// Target latency for "fast" certificate (milliseconds)
    pub target_latency_ms: u64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            decay_factor: 0.9,
            min_score: 0.1,
            target_latency_ms: 500,
        }
    }
}

/// Reputation state tracking validator performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationState {
    /// Current reputation scores for each validator (0.0 to 1.0)
    pub scores: HashMap<PublicKey, f64>,
    /// Recent performance observations (sliding window)
    pub recent_performance: VecDeque<PerformanceRecord>,
    /// Configuration parameters
    pub config: ReputationConfig,
}

impl ReputationState {
    /// Create a new reputation state with all validators at initial score
    pub fn new(validators: Vec<PublicKey>, config: ReputationConfig) -> Self {
        let scores = validators
            .into_iter()
            .map(|key| (key, 1.0)) // Start at perfect reputation
            .collect();
        
        Self {
            scores,
            recent_performance: VecDeque::new(),
            config,
        }
    }

    /// Get the reputation score for a validator
    pub fn get_score(&self, validator: &PublicKey) -> f64 {
        self.scores.get(validator).copied().unwrap_or(self.config.min_score)
    }

    /// Record a performance observation
    pub fn record_performance(&mut self, record: PerformanceRecord) {
        self.recent_performance.push_back(record);
        
        // Maintain window size
        while self.recent_performance.len() > self.config.window_size {
            self.recent_performance.pop_front();
        }
    }

    /// Calculate performance score for a single round
    pub fn calculate_round_performance(&self, latency_ms: u64, success: bool) -> f64 {
        if !success {
            return 0.0;
        }
        
        if latency_ms <= self.config.target_latency_ms {
            1.0 // Fast
        } else {
            0.5 // Slow but successful
        }
    }

    /// Update reputation scores based on recent performance
    pub fn update_scores(&mut self) {
        // Group records by validator
        let mut validator_records: HashMap<PublicKey, Vec<PerformanceRecord>> = HashMap::new();
        for record in &self.recent_performance {
            validator_records
                .entry(record.validator)
                .or_default()
                .push(record.clone());
        }

        // Update score for each validator
        for (validator, score) in self.scores.iter_mut() {
            if let Some(records) = validator_records.get(validator) {
                // Calculate weighted average of recent performance
                let mut weighted_sum = 0.0;
                let mut weight_sum = 0.0;
                
                for (i, record) in records.iter().enumerate() {
                    let weight = self.config.decay_factor.powi(i as i32);
                    let performance = Self::calc_performance(
                        record.latency_ms,
                        record.success,
                        self.config.target_latency_ms,
                    );
                    weighted_sum += weight * performance;
                    weight_sum += weight;
                }
                
                let avg_performance = if weight_sum > 0.0 {
                    weighted_sum / weight_sum
                } else {
                    *score
                };
                
                // Update score with decay
                *score = self.config.decay_factor * *score 
                    + (1.0 - self.config.decay_factor) * avg_performance;
                
                // Enforce minimum
                *score = score.max(self.config.min_score);
            }
        }
    }
    
    /// Helper to calculate performance without borrowing self
    fn calc_performance(latency_ms: u64, success: bool, target_latency_ms: u64) -> f64 {
        if !success {
            return 0.0;
        }
        
        if latency_ms <= target_latency_ms {
            1.0 // Fast
        } else {
            0.5 // Slow but successful
        }
    }
}

/// Consensus state for Shoal protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current round being processed
    pub current_round: u64,
    /// Anchors selected for each round (round -> certificate digest)
    pub anchors: BTreeMap<u64, CertificateDigest>,
    /// Set of committed certificate digests
    pub committed: BTreeSet<CertificateDigest>,
    /// Last round that was committed
    pub last_committed_round: u64,
}

impl ConsensusState {
    /// Create a new consensus state starting at genesis
    pub fn new() -> Self {
        Self {
            current_round: 0,
            anchors: BTreeMap::new(),
            committed: BTreeSet::new(),
            last_committed_round: 0,
        }
    }

    /// Get the anchor for a specific round
    pub fn get_anchor(&self, round: u64) -> Option<&CertificateDigest> {
        self.anchors.get(&round)
    }

    /// Set the anchor for a round
    pub fn set_anchor(&mut self, round: u64, anchor: CertificateDigest) {
        self.anchors.insert(round, anchor);
    }

    /// Check if a certificate is committed
    pub fn is_committed(&self, digest: &CertificateDigest) -> bool {
        self.committed.contains(digest)
    }

    /// Mark a certificate as committed
    pub fn commit(&mut self, digest: CertificateDigest) {
        self.committed.insert(digest);
    }

    /// Advance to the next round
    pub fn advance_round(&mut self) {
        self.current_round += 1;
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Helper to create a deterministic PeerId for testing
    fn test_peer_id(seed: u8) -> libp2p_identity::PeerId {
        use libp2p_identity::ed25519;
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes).expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        libp2p_identity::PeerId::from_public_key(&keypair.public().into())
    }

    #[test]
    fn test_reputation_state_initial_scores() {
        let validators = vec![test_peer_id(1), test_peer_id(2), test_peer_id(3)];
        let config = ReputationConfig::default();
        let state = ReputationState::new(validators, config);
        
        assert_eq!(state.get_score(&test_peer_id(1)), 1.0);
        assert_eq!(state.get_score(&test_peer_id(2)), 1.0);
        assert_eq!(state.get_score(&test_peer_id(3)), 1.0);
    }

    #[test]
    fn test_reputation_calculate_round_performance() {
        let config = ReputationConfig {
            target_latency_ms: 500,
            ..Default::default()
        };
        let state = ReputationState::new(vec![], config);
        
        // Fast and successful
        assert_eq!(state.calculate_round_performance(400, true), 1.0);
        
        // Slow but successful
        assert_eq!(state.calculate_round_performance(600, true), 0.5);
        
        // Failed
        assert_eq!(state.calculate_round_performance(400, false), 0.0);
    }

    #[test]
    fn test_reputation_record_performance() {
        let config = ReputationConfig {
            window_size: 3,
            ..Default::default()
        };
        let mut state = ReputationState::new(vec![vec![1]], config);
        
        // Add records
        for i in 0..5 {
            state.record_performance(PerformanceRecord {
                validator: test_peer_id(1),
                round: i,
                latency_ms: 100,
                success: true,
                timestamp: i * 1000,
            });
        }
        
        // Should only keep last 3 records (window_size)
        assert_eq!(state.recent_performance.len(), 3);
        assert_eq!(state.recent_performance[0].round, 2);
        assert_eq!(state.recent_performance[2].round, 4);
    }

    #[test]
    fn test_consensus_state_new() {
        let state = ConsensusState::new();
        assert_eq!(state.current_round, 0);
        assert_eq!(state.last_committed_round, 0);
        assert!(state.anchors.is_empty());
        assert!(state.committed.is_empty());
    }

    #[test]
    fn test_consensus_state_anchors() {
        let mut state = ConsensusState::new();
        let anchor = [1u8; 32];
        
        state.set_anchor(1, anchor);
        assert_eq!(state.get_anchor(1), Some(&anchor));
        assert_eq!(state.get_anchor(2), None);
    }

    #[test]
    fn test_consensus_state_commit() {
        let mut state = ConsensusState::new();
        let cert_digest = [1u8; 32];
        
        assert!(!state.is_committed(&cert_digest));
        state.commit(cert_digest);
        assert!(state.is_committed(&cert_digest));
    }

    #[test]
    fn test_consensus_state_advance_round() {
        let mut state = ConsensusState::new();
        assert_eq!(state.current_round, 0);
        
        state.advance_round();
        assert_eq!(state.current_round, 1);
        
        state.advance_round();
        assert_eq!(state.current_round, 2);
    }
}

