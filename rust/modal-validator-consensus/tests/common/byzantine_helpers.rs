/// Test utilities for Byzantine behavior testing
/// 
/// This module provides helpers for creating Byzantine (malicious) validators
/// and setting up test scenarios to verify the consensus protocol's fault tolerance.

use modal_validator_consensus::narwhal::{
    Certificate, Committee, Header, PublicKey, Validator, AggregatedSignature,
};
use modal_validator_consensus::narwhal::dag::DAG;
use modal_validator_consensus::shoal::{ReputationConfig};
use modal_validator_consensus::shoal::reputation::ReputationManager;
use modal_validator_consensus::shoal::consensus::ShoalConsensus;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create a deterministic PeerId for testing
pub fn test_peer_id(seed: u8) -> libp2p_identity::PeerId {
    use libp2p_identity::ed25519;
    let mut secret_bytes = [0u8; 32];
    secret_bytes[0] = seed;
    let secret = ed25519::SecretKey::try_from_bytes(secret_bytes).expect("valid secret key");
    let keypair = ed25519::Keypair::from(secret);
    libp2p_identity::PeerId::from_public_key(&keypair.public().into())
}

/// Create a test committee with N validators
/// 
/// # Arguments
/// * `n` - Number of validators (typically 4 for testing n=3f+1 with f=1)
/// 
/// # Returns
/// A committee with equal-stake validators
pub fn create_test_committee(n: usize) -> Committee {
    let validators: Vec<Validator> = (0..n)
        .map(|i| Validator {
            public_key: test_peer_id(i as u8 + 1),
            stake: 1,
            network_address: format!("127.0.0.1:800{}", i)
                .parse::<SocketAddr>()
                .unwrap(),
        })
        .collect();
    Committee::new(validators)
}

/// Create a test certificate
/// 
/// # Arguments
/// * `author` - Public key of the validator creating this certificate
/// * `round` - Round number
/// * `parents` - Parent certificate digests from the previous round
/// * `batch_digest` - Digest of the batch (use different values to create conflicting certs)
/// * `committee` - The committee for generating proper signers bitmap
/// 
/// # Returns
/// A certificate with simulated signatures from all validators
pub fn create_test_certificate(
    author: PublicKey,
    round: u64,
    parents: Vec<[u8; 32]>,
    batch_digest: [u8; 32],
    committee: &Committee,
) -> Certificate {
    let header = Header {
        author,
        round,
        batch_digest,
        parents,
        timestamp: 1000 + round * 1000,
    };
    
    // Create signers bitmap (simulate all validators signing)
    let signers = vec![true; committee.size()];
    
    Certificate {
        header,
        aggregated_signature: AggregatedSignature {
            signature: vec![],
        },
        signers,
    }
}

/// Create two conflicting certificates from the same author in the same round
/// 
/// This simulates an equivocation attack where a Byzantine validator creates
/// two different certificates for the same round to show different views to
/// different honest validators.
/// 
/// # Arguments
/// * `author` - The Byzantine validator's public key
/// * `round` - The round in which to equivocate
/// * `parents` - Parent certificate digests (same for both conflicting certs)
/// * `committee` - The committee
/// 
/// # Returns
/// A tuple of (cert1, cert2) where both are from the same author and round
/// but have different batch digests (making them conflicting)
pub fn create_conflicting_certificates(
    author: PublicKey,
    round: u64,
    parents: Vec<[u8; 32]>,
    committee: &Committee,
) -> (Certificate, Certificate) {
    // First certificate with batch digest of all 0s
    let cert1 = create_test_certificate(
        author.clone(),
        round,
        parents.clone(),
        [0u8; 32],
        committee,
    );
    
    // Second conflicting certificate with batch digest of all 1s
    let cert2 = create_test_certificate(
        author.clone(),
        round,
        parents,
        [1u8; 32],
        committee,
    );
    
    (cert1, cert2)
}

/// Setup a Byzantine test network with specified configuration
/// 
/// Creates a network of validators with DAG and consensus instances ready for testing.
/// 
/// # Arguments
/// * `total_validators` - Total number of validators (n = 3f+1)
/// * `byzantine_count` - Number of Byzantine validators (f)
/// 
/// # Returns
/// A tuple of (committee, dag, consensus)
pub fn setup_byzantine_network(
    total_validators: usize,
    _byzantine_count: usize,
) -> (Committee, Arc<RwLock<DAG>>, ShoalConsensus) {
    let committee = create_test_committee(total_validators);
    let dag = Arc::new(RwLock::new(DAG::new()));
    let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
    let consensus = ShoalConsensus::new(dag.clone(), reputation, committee.clone());
    
    (committee, dag, consensus)
}

/// Assert that equivocation was detected for a certificate
/// 
/// Verifies that attempting to insert an equivocating certificate into the DAG
/// returns an error indicating equivocation detection.
/// 
/// # Arguments
/// * `dag` - The DAG to test against
/// * `cert` - The potentially equivocating certificate
/// 
/// # Panics
/// If equivocation is not properly detected
pub async fn assert_equivocation_detected(dag: &Arc<RwLock<DAG>>, cert: &Certificate) {
    let dag_guard = dag.read().await;
    assert!(
        dag_guard.detect_equivocation(cert),
        "Equivocation should be detected for author {:?} in round {}",
        cert.header.author,
        cert.header.round
    );
}

/// Assert that inserting a certificate fails due to equivocation
/// 
/// Verifies that the DAG rejects equivocating certificates.
/// 
/// # Arguments
/// * `dag` - The DAG to test against
/// * `cert` - The equivocating certificate to insert
/// 
/// # Panics
/// If the insertion succeeds (equivocation not prevented)
pub async fn assert_insert_fails_equivocation(dag: &Arc<RwLock<DAG>>, cert: Certificate) {
    let mut dag_guard = dag.write().await;
    let result = dag_guard.insert(cert.clone());
    
    assert!(
        result.is_err(),
        "Insert should fail for equivocating certificate from {:?} in round {}",
        cert.header.author,
        cert.header.round
    );
    
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("equivocation"),
        "Error message should mention equivocation, got: {}",
        err_msg
    );
}

/// Create a Byzantine validator configuration
/// 
/// This is a marker struct that can be extended to configure specific
/// Byzantine behaviors for testing.
#[allow(dead_code)]
pub struct ByzantineValidator {
    pub public_key: PublicKey,
    pub behavior: ByzantineBehavior,
}

/// Types of Byzantine behavior to simulate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ByzantineBehavior {
    /// Create two conflicting certificates in the same round
    Equivocate,
    /// Refuse to vote for others' certificates
    WithholdVotes,
    /// Refuse to broadcast own certificates
    WithholdCertificates,
    /// Completely silent (no participation)
    Silent,
}

#[allow(dead_code)]
impl ByzantineValidator {
    pub fn new(seed: u8, behavior: ByzantineBehavior) -> Self {
        Self {
            public_key: test_peer_id(seed),
            behavior,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_committee() {
        let committee = create_test_committee(4);
        assert_eq!(committee.size(), 4);
        assert_eq!(committee.quorum_threshold(), 3); // 2f+1 where f=1
    }

    #[test]
    fn test_create_conflicting_certificates() {
        let committee = create_test_committee(4);
        let author = test_peer_id(1);
        
        let (cert1, cert2) = create_conflicting_certificates(
            author,
            0,
            vec![],
            &committee,
        );
        
        // Same author and round
        assert_eq!(cert1.header.author, cert2.header.author);
        assert_eq!(cert1.header.round, cert2.header.round);
        
        // Different batch digests (conflicting)
        assert_ne!(cert1.header.batch_digest, cert2.header.batch_digest);
        
        // Different certificate digests
        assert_ne!(cert1.digest(), cert2.digest());
    }

    #[tokio::test]
    async fn test_setup_byzantine_network() {
        let (committee, dag, _consensus) = setup_byzantine_network(4, 1);
        
        assert_eq!(committee.size(), 4);
        
        let dag_guard = dag.read().await;
        assert_eq!(dag_guard.highest_round(), 0);
    }
}

