use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use libp2p_identity::PeerId;

/// Type aliases for cryptographic primitives
pub type PublicKey = PeerId; // Libp2p peer identifier
pub type Signature = Vec<u8>;
pub type Digest = [u8; 32]; // SHA-256
pub type BatchDigest = Digest;
pub type CertificateDigest = Digest;
pub type WorkerId = u32;

/// A transaction to be ordered by consensus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// A batch of transactions collected by a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Transactions in this batch
    pub transactions: Vec<Transaction>,
    /// Worker ID that created this batch
    pub worker_id: WorkerId,
    /// Timestamp of batch creation
    pub timestamp: u64,
}

impl Batch {
    /// Compute the digest of this batch
    pub fn digest(&self) -> BatchDigest {
        use sha2::{Digest as _, Sha256};
        let serialized = bincode::serialize(self).expect("serialization should not fail");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        hasher.finalize().into()
    }
}

/// Header metadata about a batch, proposed by a primary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Author (validator public key)
    pub author: PublicKey,
    /// Round number (monotonically increasing)
    pub round: u64,
    /// Digest of the batch this header references
    pub batch_digest: BatchDigest,
    /// References to certificates from previous round (parents)
    pub parents: Vec<CertificateDigest>,
    /// Timestamp of header creation
    pub timestamp: u64,
}

impl Header {
    /// Compute the digest of this header
    pub fn digest(&self) -> Digest {
        use sha2::{Digest as _, Sha256};
        let serialized = bincode::serialize(self).expect("serialization should not fail");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        hasher.finalize().into()
    }

    /// Verify that parent references are valid for this round
    pub fn verify_parents(&self, expected_round: u64) -> anyhow::Result<()> {
        if self.round != expected_round {
            anyhow::bail!("header round {} does not match expected {}", self.round, expected_round);
        }

        // Round 0 (genesis) should have no parents
        if self.round == 0 && !self.parents.is_empty() {
            anyhow::bail!("genesis header should have no parents");
        }

        // Non-genesis rounds must have parents
        if self.round > 0 && self.parents.is_empty() {
            anyhow::bail!("non-genesis header must have parents");
        }

        Ok(())
    }
}

/// Aggregated signature from multiple validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSignature {
    /// The aggregated signature bytes
    pub signature: Vec<u8>,
}

/// Bitmap indicating which validators signed
pub type BitVec = Vec<bool>;

/// A certificate: header + 2f+1 signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// The header being certified
    pub header: Header,
    /// Aggregated signature from validators
    pub aggregated_signature: AggregatedSignature,
    /// Bitmap indicating which validators signed (by index in committee)
    pub signers: BitVec,
}

impl Certificate {
    /// Compute the digest of this certificate
    pub fn digest(&self) -> CertificateDigest {
        // Certificate digest is same as header digest
        // (since certificate is just header + signatures)
        self.header.digest()
    }

    /// Check if this certificate has quorum (2f+1 signatures)
    /// This uses a simple count-based check. For stake-weighted quorum,
    /// use has_quorum_weighted() with a Committee.
    pub fn has_quorum(&self, total_validators: usize) -> bool {
        let signature_count = self.signers.iter().filter(|&&signed| signed).count();
        let threshold = crate::consensus_math::calculate_2f_plus_1(total_validators as f64);
        signature_count >= threshold as usize
    }
    
    /// Check if this certificate has quorum with stake-weighted voting
    pub fn has_quorum_weighted(&self, committee: &Committee) -> bool {
        // Get the public keys of all signers
        let signer_keys: Vec<PublicKey> = self.get_signer_indices()
            .into_iter()
            .filter_map(|idx| committee.validator_order.get(idx).cloned())
            .collect();
        
        committee.check_quorum(&signer_keys)
    }

    /// Get the list of signers' indices
    pub fn get_signer_indices(&self) -> Vec<usize> {
        self.signers
            .iter()
            .enumerate()
            .filter_map(|(idx, &signed)| if signed { Some(idx) } else { None })
            .collect()
    }
}

/// Information about a validator in the committee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// Validator's public key
    pub public_key: PublicKey,
    /// Validator's stake (weight in voting)
    pub stake: u64,
    /// Network address for communication
    pub network_address: SocketAddr,
}

/// Committee: the set of validators participating in consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Committee {
    /// Map from public key to validator info
    pub validators: HashMap<PublicKey, Validator>,
    /// Ordered list of public keys (for indexing in bitvecs)
    pub validator_order: Vec<PublicKey>,
}

impl Committee {
    /// Create a new committee from a list of validators
    pub fn new(validators: Vec<Validator>) -> Self {
        let validator_order: Vec<PublicKey> = validators.iter().map(|v| v.public_key).collect();
        let validators: HashMap<PublicKey, Validator> = validators
            .into_iter()
            .map(|v| (v.public_key, v))
            .collect();
        
        Self {
            validators,
            validator_order,
        }
    }

    /// Get the total number of validators
    pub fn size(&self) -> usize {
        self.validators.len()
    }
    
    /// Get the total stake in the committee
    pub fn total_stake(&self) -> u64 {
        self.validators.values().map(|v| v.stake).sum()
    }

    /// Get the quorum threshold (2f+1) - stake-weighted
    /// 
    /// If all validators have equal stake (1), this is equivalent to the count-based threshold.
    /// If stakes vary, this returns 2f+1 of the total stake.
    pub fn quorum_threshold(&self) -> u64 {
        let total = self.total_stake();
        crate::consensus_math::calculate_2f_plus_1(total as f64)
    }
    
    /// Get the quorum threshold based on validator count (old behavior)
    /// 
    /// This is kept for compatibility but quorum_threshold() is preferred
    /// as it properly accounts for stake-weighted voting.
    pub fn quorum_threshold_by_count(&self) -> u64 {
        crate::consensus_math::calculate_2f_plus_1(self.size() as f64)
    }

    /// Get the maximum number of Byzantine validators tolerated (f)
    pub fn max_byzantine(&self) -> usize {
        (self.size() - 1) / 3
    }
    
    /// Check if a set of votes meets the quorum threshold (stake-weighted)
    pub fn check_quorum(&self, voters: &[PublicKey]) -> bool {
        let stake_voted: u64 = voters.iter()
            .filter_map(|pk| self.validators.get(pk).map(|v| v.stake))
            .sum();
        stake_voted >= self.quorum_threshold()
    }
    
    /// Get the total stake represented by a set of validators
    pub fn get_stake(&self, validators: &[PublicKey]) -> u64 {
        validators.iter()
            .filter_map(|pk| self.validators.get(pk).map(|v| v.stake))
            .sum()
    }

    /// Get validator by public key
    pub fn get_validator(&self, key: &PublicKey) -> Option<&Validator> {
        self.validators.get(key)
    }

    /// Get the index of a validator in the ordered list
    pub fn get_index(&self, key: &PublicKey) -> Option<usize> {
        self.validator_order.iter().position(|k| k == key)
    }

    /// Check if a public key is in the committee
    pub fn contains(&self, key: &PublicKey) -> bool {
        self.validators.contains_key(key)
    }
}

/// A vote on a header from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// The header being voted on
    pub header_digest: Digest,
    /// Round number
    pub round: u64,
    /// Voter's public key
    pub voter: PublicKey,
    /// Signature on header_digest
    pub signature: Signature,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Helper to create a deterministic PeerId for testing
    fn test_peer_id(seed: u8) -> PeerId {
        use libp2p_identity::ed25519;
        
        // Create a deterministic keypair from a seed
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
            .expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        PeerId::from_public_key(&keypair.public().into())
    }

    #[test]
    fn test_batch_digest() {
        let batch = Batch {
            transactions: vec![
                Transaction {
                    data: vec![1, 2, 3],
                    timestamp: 100,
                },
            ],
            worker_id: 0,
            timestamp: 1000,
        };
        
        let digest1 = batch.digest();
        let digest2 = batch.digest();
        assert_eq!(digest1, digest2, "digest should be deterministic");
    }

    #[test]
    fn test_header_verify_parents_genesis() {
        let header = Header {
            author: test_peer_id(1),
            round: 0,
            batch_digest: [0u8; 32],
            parents: vec![],
            timestamp: 1000,
        };
        
        assert!(header.verify_parents(0).is_ok());
        
        // Genesis with parents should fail
        let bad_header = Header {
            parents: vec![[1u8; 32]],
            ..header.clone()
        };
        assert!(bad_header.verify_parents(0).is_err());
    }

    #[test]
    fn test_header_verify_parents_non_genesis() {
        let header = Header {
            author: test_peer_id(1),
            round: 1,
            batch_digest: [0u8; 32],
            parents: vec![[1u8; 32], [2u8; 32]],
            timestamp: 1000,
        };
        
        assert!(header.verify_parents(1).is_ok());
        
        // Non-genesis without parents should fail
        let bad_header = Header {
            parents: vec![],
            ..header.clone()
        };
        assert!(bad_header.verify_parents(1).is_err());
    }

    #[test]
    fn test_certificate_has_quorum() {
        // 4 validators, need 3 signatures (2f+1 where f=1)
        let cert = Certificate {
            header: Header {
                author: test_peer_id(1),
                round: 1,
                batch_digest: [0u8; 32],
                parents: vec![],
                timestamp: 1000,
            },
            aggregated_signature: AggregatedSignature {
                signature: vec![],
            },
            signers: vec![true, true, true, false], // 3 signers
        };
        
        assert!(cert.has_quorum(4));
        
        // Only 2 signers - no quorum
        let cert_no_quorum = Certificate {
            signers: vec![true, true, false, false],
            ..cert.clone()
        };
        assert!(!cert_no_quorum.has_quorum(4));
    }

    #[test]
    fn test_committee_quorum_threshold() {
        let validators = vec![
            Validator {
                public_key: test_peer_id(1),
                stake: 1,
                network_address: "127.0.0.1:8000".parse().unwrap(),
            },
            Validator {
                public_key: test_peer_id(2),
                stake: 1,
                network_address: "127.0.0.1:8001".parse().unwrap(),
            },
            Validator {
                public_key: test_peer_id(3),
                stake: 1,
                network_address: "127.0.0.1:8002".parse().unwrap(),
            },
            Validator {
                public_key: test_peer_id(4),
                stake: 1,
                network_address: "127.0.0.1:8003".parse().unwrap(),
            },
        ];
        
        let committee = Committee::new(validators);
        assert_eq!(committee.size(), 4);
        assert_eq!(committee.quorum_threshold(), 3); // 2*4/3 + 1 = 3
        assert_eq!(committee.max_byzantine(), 1); // (4-1)/3 = 1
    }

    #[test]
    fn test_committee_get_index() {
        let validators = vec![
            Validator {
                public_key: test_peer_id(1),
                stake: 1,
                network_address: "127.0.0.1:8000".parse().unwrap(),
            },
            Validator {
                public_key: test_peer_id(2),
                stake: 1,
                network_address: "127.0.0.1:8001".parse().unwrap(),
            },
        ];
        
        let committee = Committee::new(validators);
        assert_eq!(committee.get_index(&test_peer_id(1)), Some(0));
        assert_eq!(committee.get_index(&test_peer_id(2)), Some(1));
        assert_eq!(committee.get_index(&test_peer_id(99)), None);
    }
}

