use crate::narwhal::{Batch, Certificate, CertificateDigest, PublicKey};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "persistence")]
use modal_datastore::NetworkDatastore;
#[cfg(feature = "persistence")]
use crate::persistence::{ToPersistenceModel, digest_to_hex};

/// DAG storage and management
#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct DAG {
    /// Primary storage: digest -> certificate
    certificates: HashMap<CertificateDigest, Certificate>,
    
    /// Index by round for efficient queries
    by_round: BTreeMap<u64, Vec<CertificateDigest>>,
    
    /// Index by author: author -> (round -> digest)
    by_author: HashMap<PublicKey, BTreeMap<u64, CertificateDigest>>,
}

impl std::fmt::Debug for DAG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DAG")
            .field("certificate_count", &self.certificates.len())
            .field("highest_round", &self.highest_round())
            .field("rounds", &self.by_round.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl DAG {
    /// Create a new empty DAG
    pub fn new() -> Self {
        Self {
            certificates: HashMap::new(),
            by_round: BTreeMap::new(),
            by_author: HashMap::new(),
        }
    }

    /// Insert a certificate into the DAG
    pub fn insert(&mut self, cert: Certificate) -> Result<()> {
        let digest = cert.digest();
        let round = cert.header.round;
        let author = cert.header.author.clone();

        // Check for equivocation
        if self.detect_equivocation(&cert) {
            bail!("equivocation detected for author {:?} in round {}", author, round);
        }

        // Verify parents exist (except genesis)
        if round > 0 {
            for parent_digest in &cert.header.parents {
                if !self.certificates.contains_key(parent_digest) {
                    bail!("parent certificate not found: {:?}", parent_digest);
                }
            }
        }

        // Insert into primary storage
        self.certificates.insert(digest, cert.clone());

        // Update round index
        self.by_round
            .entry(round)
            .or_default()
            .push(digest);

        // Update author index
        self.by_author
            .entry(author.clone())
            .or_default()
            .insert(round, digest);

        log::debug!("inserted certificate {} for round {} from {:?}", hex::encode(&digest), round, &author);
        
        Ok(())
    }

    /// Get a certificate by digest
    pub fn get(&self, digest: &CertificateDigest) -> Option<&Certificate> {
        self.certificates.get(digest)
    }

    /// Get all certificates in a specific round
    pub fn get_round(&self, round: u64) -> Vec<&Certificate> {
        self.by_round
            .get(&round)
            .map(|digests| {
                digests
                    .iter()
                    .filter_map(|d| self.certificates.get(d))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a certificate for a specific author in a specific round
    pub fn get_author_cert(&self, author: &PublicKey, round: u64) -> Option<&Certificate> {
        self.by_author
            .get(author)?
            .get(&round)
            .and_then(|digest| self.certificates.get(digest))
    }

    /// Check if there is a path from `from` certificate to `to` certificate
    pub fn has_path(&self, from: &CertificateDigest, to: &CertificateDigest) -> bool {
        if from == to {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![*from];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if current == *to {
                return true;
            }

            if let Some(cert) = self.certificates.get(&current) {
                for parent in &cert.header.parents {
                    if !visited.contains(parent) {
                        stack.push(*parent);
                    }
                }
            }
        }

        false
    }

    /// Detect if a certificate is an equivocation
    /// (i.e., author already has a different certificate in this round)
    pub fn detect_equivocation(&self, cert: &Certificate) -> bool {
        if let Some(existing_digest) = self.by_author
            .get(&cert.header.author)
            .and_then(|rounds| rounds.get(&cert.header.round))
        {
            // Equivocation if digest differs
            *existing_digest != cert.digest()
        } else {
            false
        }
    }

    /// Get the current highest round in the DAG
    pub fn highest_round(&self) -> u64 {
        self.by_round
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
    }

    /// Get the number of certificates in a round
    pub fn round_size(&self, round: u64) -> usize {
        self.by_round
            .get(&round)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get all rounds that have certificates
    pub fn rounds(&self) -> Vec<u64> {
        self.by_round.keys().copied().collect()
    }

    /// Get all certificates at a specific round
    pub fn certificates_at_round(&self, round: u64) -> Vec<&Certificate> {
        self.get_round(round)
    }

    /// Get a certificate by digest (alias for get)
    pub fn get_certificate(&self, digest: &CertificateDigest) -> Option<&Certificate> {
        self.get(digest)
    }

    // Persistence methods
    #[cfg(feature = "persistence")]
    pub async fn persist_certificate(
        &self,
        cert: &Certificate,
        datastore: &NetworkDatastore,
    ) -> Result<()> {
        use modal_datastore::Model;
        let model = cert.to_persistence_model()?;
        model.save(datastore).await?;
        Ok(())
    }

    #[cfg(feature = "persistence")]
    pub async fn persist_batch(
        &self,
        batch: &Batch,
        author: &PublicKey,
        cert_digest: Option<&CertificateDigest>,
        datastore: &NetworkDatastore,
    ) -> Result<()> {
        use modal_datastore::Model;
        let mut model = batch.to_persistence_model()?;
        model.author = author.to_base58();
        if let Some(digest) = cert_digest {
            model.referenced_by_cert = Some(digest_to_hex(digest));
        }
        model.save(datastore).await?;
        Ok(())
    }

    #[cfg(feature = "persistence")]
    pub async fn load_from_datastore(datastore: &NetworkDatastore) -> Result<Self> {
        use crate::persistence::recovery::{recover_dag, RecoveryStrategy};
        
        let result = recover_dag(datastore, RecoveryStrategy::FromScratch).await?;
        Ok(result.dag)
    }

    #[cfg(feature = "persistence")]
    pub async fn create_checkpoint(
        &self,
        round: u64,
        consensus_state: &crate::shoal::ConsensusState,
        reputation_state: &crate::shoal::ReputationState,
        datastore: &NetworkDatastore,
    ) -> Result<()> {
        use modal_datastore::models::DAGState;
        use modal_datastore::Model;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Serialize DAG
        let dag_bytes = bincode::serialize(self)?;
        use base64::Engine;
        let dag_snapshot = base64::engine::general_purpose::STANDARD.encode(&dag_bytes);
        
        let checkpoint = DAGState {
            checkpoint_round: round,
            checkpoint_id: uuid::Uuid::new_v4().to_string(),
            highest_round: self.highest_round(),
            certificate_count: self.certificates.len(),
            committed_count: consensus_state.committed.len(),
            dag_snapshot,
            consensus_state: serde_json::to_string(consensus_state)?,
            reputation_state: serde_json::to_string(reputation_state)?,
            created_at: now,
            size_bytes: dag_bytes.len(),
        };
        
        checkpoint.save(datastore).await?;
        log::info!("Created checkpoint at round {} ({} bytes)", round, dag_bytes.len());
        
        Ok(())
    }

    #[cfg(feature = "persistence")]
    pub async fn load_from_checkpoint(datastore: &NetworkDatastore) -> Result<Self> {
        use crate::persistence::recovery::{recover_dag, RecoveryStrategy};
        
        let result = recover_dag(datastore, RecoveryStrategy::FromCheckpoint).await?;
        Ok(result.dag)
    }
}

impl Default for DAG {
    fn default() -> Self {
        Self::new()
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
    use crate::narwhal::{AggregatedSignature, Header, PublicKey};

    fn make_test_cert(author: PublicKey, round: u64, parents: Vec<CertificateDigest>) -> Certificate {
        Certificate {
            header: Header {
                author,
                round,
                batch_digest: [0u8; 32],
                parents,
                timestamp: 1000,
            },
            aggregated_signature: AggregatedSignature {
                signature: vec![],
            },
            signers: vec![true, true, true, false], // 3 signers
        }
    }

    #[test]
    fn test_dag_insert_genesis() {
        let mut dag = DAG::new();
        let cert = make_test_cert(test_peer_id(1), 0, vec![]);
        
        assert!(dag.insert(cert).is_ok());
        assert_eq!(dag.highest_round(), 0);
        assert_eq!(dag.round_size(0), 1);
    }

    #[test]
    fn test_dag_insert_with_parents() {
        let mut dag = DAG::new();
        
        // Insert genesis
        let genesis = make_test_cert(test_peer_id(1), 0, vec![]);
        let genesis_digest = genesis.digest();
        dag.insert(genesis).unwrap();
        
        // Insert round 1 with parent
        let cert1 = make_test_cert(test_peer_id(2), 1, vec![genesis_digest]);
        assert!(dag.insert(cert1).is_ok());
        assert_eq!(dag.highest_round(), 1);
    }

    #[test]
    fn test_dag_insert_missing_parent() {
        let mut dag = DAG::new();
        
        // Try to insert round 1 without parent in DAG
        let cert = make_test_cert(test_peer_id(1), 1, vec![[1u8; 32]]);
        assert!(dag.insert(cert).is_err());
    }

    #[test]
    fn test_dag_detect_equivocation() {
        let mut dag = DAG::new();
        
        let cert1 = make_test_cert(test_peer_id(1), 0, vec![]);
        dag.insert(cert1).unwrap();
        
        // Try to insert different cert from same author in same round
        let mut cert2 = make_test_cert(test_peer_id(1), 0, vec![]);
        cert2.header.batch_digest = [1u8; 32]; // Different batch
        
        assert!(dag.detect_equivocation(&cert2));
        assert!(dag.insert(cert2).is_err());
    }

    #[test]
    fn test_dag_has_path() {
        let mut dag = DAG::new();
        
        // Build chain: 0 -> 1 -> 2
        let cert0 = make_test_cert(test_peer_id(1), 0, vec![]);
        let digest0 = cert0.digest();
        dag.insert(cert0).unwrap();
        
        let cert1 = make_test_cert(test_peer_id(2), 1, vec![digest0]);
        let digest1 = cert1.digest();
        dag.insert(cert1).unwrap();
        
        let cert2 = make_test_cert(test_peer_id(3), 2, vec![digest1]);
        let digest2 = cert2.digest();
        dag.insert(cert2).unwrap();
        
        // Test paths
        assert!(dag.has_path(&digest2, &digest1));
        assert!(dag.has_path(&digest2, &digest0));
        assert!(dag.has_path(&digest1, &digest0));
        assert!(!dag.has_path(&digest0, &digest1));
    }

    #[test]
    fn test_dag_get_author_cert() {
        let mut dag = DAG::new();
        
        let author = test_peer_id(1);
        let cert = make_test_cert(author.clone(), 0, vec![]);
        dag.insert(cert).unwrap();
        
        assert!(dag.get_author_cert(&author, 0).is_some());
        assert!(dag.get_author_cert(&author, 1).is_none());
        assert!(dag.get_author_cert(&vec![2], 0).is_none());
    }
}

