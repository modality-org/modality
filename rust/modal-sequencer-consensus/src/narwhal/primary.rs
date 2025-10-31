use crate::narwhal::{BatchDigest, Certificate, CertificateDigest, Committee, Header, PublicKey};
use crate::narwhal::certificate::CertificateBuilder;
use crate::narwhal::dag::DAG;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Primary node that creates headers and forms certificates
pub struct Primary {
    /// This validator's public key
    pub validator: PublicKey,
    /// Committee of all validators
    pub committee: Committee,
    /// The DAG
    pub dag: Arc<RwLock<DAG>>,
    /// Current round
    pub current_round: u64,
}

impl Primary {
    /// Create a new primary
    pub fn new(
        validator: PublicKey,
        committee: Committee,
        dag: Arc<RwLock<DAG>>,
    ) -> Self {
        Self {
            validator,
            committee,
            dag,
            current_round: 0,
        }
    }

    /// Propose a new header for the current round
    pub async fn propose(
        &mut self,
        batch_digest: BatchDigest,
    ) -> Result<Header> {
        let dag = self.dag.read().await;
        
        // Get parents from previous round
        let prev_round = self.current_round.saturating_sub(1);
        let parents: Vec<CertificateDigest> = if self.current_round == 0 {
            // Genesis has no parents
            vec![]
        } else {
            // Reference certificates from previous round
            dag.get_round(prev_round)
                .iter()
                .map(|cert| cert.digest())
                .collect()
        };

        // Verify we have enough parents (2f+1) for non-genesis
        if self.current_round > 0 {
            let quorum = self.committee.quorum_threshold();
            if parents.len() < quorum as usize {
                anyhow::bail!(
                    "insufficient parents: {} < {}",
                    parents.len(),
                    quorum
                );
            }
        }

        let header = Header {
            author: self.validator.clone(),
            round: self.current_round,
            batch_digest,
            parents,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        Ok(header)
    }

    /// Create a certificate builder for a header
    pub fn create_certificate_builder(&self, header: Header) -> CertificateBuilder {
        CertificateBuilder::new(header, self.committee.clone())
    }

    /// Process a certificate and add it to the DAG
    pub async fn process_certificate(&self, cert: Certificate) -> Result<()> {
        let mut dag = self.dag.write().await;
        dag.insert(cert)?;
        Ok(())
    }

    /// Advance to the next round
    pub fn advance_round(&mut self) {
        self.current_round += 1;
        log::info!("primary advanced to round {}", self.current_round);
    }

    /// Get the current round
    pub fn get_current_round(&self) -> u64 {
        self.current_round
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
    use crate::narwhal::{AggregatedSignature, Validator};
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

    #[tokio::test]
    async fn test_primary_propose_genesis() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let mut primary = Primary::new(test_peer_id(1), committee, dag);

        let batch_digest = [0u8; 32];
        let header = primary.propose(batch_digest).await.unwrap();

        assert_eq!(header.round, 0);
        assert_eq!(header.author, vec![1]);
        assert!(header.parents.is_empty());
    }

    #[tokio::test]
    async fn test_primary_propose_with_parents() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let mut primary = Primary::new(test_peer_id(1), committee, dag.clone());

        // Add genesis certificates to DAG
        for i in 1..=4 {
            let cert = Certificate {
                header: Header {
                    author: vec![i],
                    round: 0,
                    batch_digest: [0u8; 32],
                    parents: vec![],
                    timestamp: 1000,
                },
                aggregated_signature: AggregatedSignature {
                    signature: vec![],
                },
                signers: vec![true, true, true, false],
            };
            dag.write().await.insert(cert).unwrap();
        }

        // Advance to round 1
        primary.advance_round();

        let batch_digest = [1u8; 32];
        let header = primary.propose(batch_digest).await.unwrap();

        assert_eq!(header.round, 1);
        assert_eq!(header.parents.len(), 4); // All genesis certificates
    }

    #[tokio::test]
    async fn test_primary_process_certificate() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let primary = Primary::new(test_peer_id(1), committee, dag.clone());

        let cert = Certificate {
            header: Header {
                author: test_peer_id(1),
                round: 0,
                batch_digest: [0u8; 32],
                parents: vec![],
                timestamp: 1000,
            },
            aggregated_signature: AggregatedSignature {
                signature: vec![],
            },
            signers: vec![true, true, true, false],
        };

        let digest = cert.digest();
        primary.process_certificate(cert).await.unwrap();

        let dag_read = dag.read().await;
        assert!(dag_read.get(&digest).is_some());
    }

    #[tokio::test]
    async fn test_primary_advance_round() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let mut primary = Primary::new(test_peer_id(1), committee, dag);

        assert_eq!(primary.get_current_round(), 0);
        
        primary.advance_round();
        assert_eq!(primary.get_current_round(), 1);
        
        primary.advance_round();
        assert_eq!(primary.get_current_round(), 2);
    }
}

