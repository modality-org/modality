use crate::narwhal::{Certificate, CertificateDigest, Committee};
use crate::narwhal::dag::DAG;
use crate::shoal::{ConsensusState, PerformanceRecord};
use crate::shoal::reputation::ReputationManager;
use anyhow::{bail, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "persistence")]
use modal_datastore::DatastoreManager;

/// Shoal consensus engine
pub struct ShoalConsensus {
    /// The DAG
    pub dag: Arc<RwLock<DAG>>,
    /// Reputation manager for leader selection
    pub reputation: ReputationManager,
    /// Consensus state
    pub state: ConsensusState,
    /// Committee
    pub committee: Committee,
    /// Optional datastore for persistence
    #[cfg(feature = "persistence")]
    pub datastore: Option<Arc<DatastoreManager>>,
}

impl ShoalConsensus {
    /// Create a new Shoal consensus instance
    pub fn new(
        dag: Arc<RwLock<DAG>>,
        reputation: ReputationManager,
        committee: Committee,
    ) -> Self {
        Self {
            dag,
            reputation,
            state: ConsensusState::new(),
            committee,
            #[cfg(feature = "persistence")]
            datastore: None,
        }
    }

    /// Set the datastore for persistence
    #[cfg(feature = "persistence")]
    pub fn with_datastore(mut self, datastore: Arc<DatastoreManager>) -> Self {
        self.datastore = Some(datastore);
        self
    }

    /// Process a new certificate and potentially commit
    pub async fn process_certificate(&mut self, cert: Certificate) -> Result<Vec<CertificateDigest>> {
        let round = cert.header.round;
        let digest = cert.digest();
        
        log::debug!("processing certificate {} for round {}", hex::encode(&digest), round);

        // Add to DAG (if not already there)
        {
            let mut dag = self.dag.write().await;
            if dag.get(&digest).is_none() {
                dag.insert(cert.clone())?;
            }
        }

        // Update reputation based on certificate arrival
        self.record_certificate_performance(&cert);

        // Try to select anchor for this round
        if let Some(anchor) = self.try_select_anchor(round).await? {
            log::info!("selected anchor {} for round {}", hex::encode(&anchor), round);
            self.state.set_anchor(round, anchor);

            // Check commit rule
            if self.check_commit_rule(&anchor).await? {
                log::info!("committing anchor {} for round {}", hex::encode(&anchor), round);
                return self.commit_certificate(anchor).await;
            }
        }

        Ok(vec![])
    }

    /// Try to select an anchor for a round
    async fn try_select_anchor(&self, round: u64) -> Result<Option<CertificateDigest>> {
        // Check if we already have an anchor for this round
        if self.state.get_anchor(round).is_some() {
            return Ok(None);
        }

        let dag = self.dag.read().await;

        // Select leader based on reputation
        let leader = self.reputation.select_leader(round);

        // Try to get leader's certificate for this round
        if let Some(leader_cert) = dag.get_author_cert(&leader, round) {
            return Ok(Some(leader_cert.digest()));
        }

        // Fallback: if leader's certificate not available, select next-best
        // (implements prevalent responsiveness)
        let certs = dag.get_round(round);
        if !certs.is_empty() {
            // Select based on reputation of available certificates
            let mut cert_scores: Vec<(CertificateDigest, f64)> = certs
                .iter()
                .map(|cert| {
                    let score = self.reputation.get_score(&cert.header.author);
                    (cert.digest(), score)
                })
                .collect();

            cert_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((digest, _)) = cert_scores.first() {
                log::info!("using fallback anchor (leader unavailable) for round {}", round);
                return Ok(Some(*digest));
            }
        }

        Ok(None)
    }

    /// Check if an anchor satisfies the commit rule
    async fn check_commit_rule(&self, anchor: &CertificateDigest) -> Result<bool> {
        let dag = self.dag.read().await;
        
        let anchor_cert = dag.get(anchor)
            .ok_or_else(|| anyhow::anyhow!("anchor certificate not found"))?;
        
        let current_round = anchor_cert.header.round;

        // Genesis can commit immediately
        if current_round == 0 {
            return Ok(true);
        }

        // For round > 0: need path to 2f+1 anchors from previous round
        let prev_round = current_round - 1;

        // Get anchors from previous round
        let prev_anchors: Vec<CertificateDigest> = (0..=prev_round)
            .filter_map(|r| self.state.get_anchor(r).copied())
            .collect();

        if prev_anchors.is_empty() {
            // No previous anchors yet
            return Ok(false);
        }

        // Count how many previous anchors are reachable
        let mut reachable_count = 0;
        for prev_anchor in &prev_anchors {
            if dag.has_path(anchor, prev_anchor) {
                reachable_count += 1;
            }
        }

        let quorum = self.committee.quorum_threshold() as usize;
        Ok(reachable_count >= quorum)
    }

    /// Commit a certificate and return all newly committed certificates
    async fn commit_certificate(&mut self, anchor: CertificateDigest) -> Result<Vec<CertificateDigest>> {
        let dag = self.dag.read().await;
        
        let anchor_cert = dag.get(&anchor)
            .ok_or_else(|| anyhow::anyhow!("anchor certificate not found"))?;
        
        let round = anchor_cert.header.round;

        // Mark anchor as committed
        self.state.commit(anchor);
        self.state.last_committed_round = round;

        // Collect all certificates that are now committed (causal history)
        let mut newly_committed = vec![anchor];
        let mut to_process = vec![anchor];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = to_process.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(cert) = dag.get(&current) {
                for parent in &cert.header.parents {
                    if !self.state.is_committed(parent) && !visited.contains(parent) {
                        self.state.commit(*parent);
                        newly_committed.push(*parent);
                        to_process.push(*parent);
                    }
                }
            }
        }

        log::info!("committed {} certificates (anchor round {})", newly_committed.len(), round);

        // Persist committed certificates to datastore
        #[cfg(feature = "persistence")]
        if let Some(datastore) = &self.datastore {
            use modal_datastore::models::DAGCertificate;
            use crate::persistence::digest_to_hex;
            
            for digest in &newly_committed {
                let digest_hex = digest_to_hex(digest);
                let cert_round = dag.get(digest).map(|c| c.header.round).unwrap_or(0);
                
                // Load certificate model and mark as committed
                let keys = [
                    ("round".to_string(), cert_round.to_string()),
                    ("digest".to_string(), digest_hex),
                ].into_iter().collect();
                
                if let Ok(Some(mut cert_model)) = DAGCertificate::find_one_multi(datastore, keys).await {
                    if cert_model.mark_committed_multi(datastore, round).await.is_err() {
                        log::warn!("failed to mark certificate {:?} as committed", digest);
                    }
                }
            }
        }

        Ok(newly_committed)
    }

    /// Record certificate performance for reputation
    fn record_certificate_performance(&mut self, cert: &Certificate) {
        // TODO: Calculate actual latency from round start
        // For now, assume fast certificates (this would be replaced with real timing)
        let latency_ms = 500; // Placeholder

        self.reputation.record_performance(PerformanceRecord {
            validator: cert.header.author.clone(),
            round: cert.header.round,
            latency_ms,
            success: true,
            timestamp: cert.header.timestamp,
        });
    }

    /// Advance to the next round
    pub fn advance_round(&mut self) {
        self.state.advance_round();
        // Update reputation scores periodically
        if self.state.current_round % 10 == 0 {
            self.reputation.update_scores();
        }
    }

    /// Get the current consensus round
    pub fn current_round(&self) -> u64 {
        self.state.current_round
    }

    /// Get the last committed round
    pub fn last_committed_round(&self) -> u64 {
        self.state.last_committed_round
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
    use crate::narwhal::{AggregatedSignature, Header, Validator};
    use crate::shoal::ReputationConfig;
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

    fn make_test_cert(author: crate::narwhal::PublicKey, round: u64, parents: Vec<CertificateDigest>) -> Certificate {
        Certificate {
            header: Header {
                author,
                round,
                batch_digest: [0u8; 32],
                parents,
                timestamp: 1000 + round * 1000,
            },
            aggregated_signature: AggregatedSignature {
                signature: vec![],
            },
            signers: vec![true, true, true, false],
        }
    }

    #[tokio::test]
    async fn test_shoal_consensus_process_genesis() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
        let mut consensus = ShoalConsensus::new(dag, reputation, committee);

        let cert = make_test_cert(vec![1], 0, vec![]);
        let committed = consensus.process_certificate(cert).await.unwrap();

        // Genesis should commit immediately
        assert!(!committed.is_empty());
        assert_eq!(consensus.last_committed_round(), 0);
    }

    #[tokio::test]
    async fn test_shoal_consensus_advance_round() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
        let mut consensus = ShoalConsensus::new(dag, reputation, committee);

        assert_eq!(consensus.current_round(), 0);
        
        consensus.advance_round();
        assert_eq!(consensus.current_round(), 1);
    }

    #[tokio::test]
    async fn test_shoal_consensus_anchor_selection() {
        let committee = make_test_committee();
        let dag = Arc::new(RwLock::new(DAG::new()));
        let reputation = ReputationManager::new(committee.clone(), ReputationConfig::default());
        let mut consensus = ShoalConsensus::new(dag.clone(), reputation, committee);

        // Add genesis certificates and process them
        for i in 1..=4 {
            let cert = make_test_cert(vec![i], 0, vec![]);
            consensus.process_certificate(cert).await.unwrap();
        }

        // We should have an anchor now for round 0
        let anchor = consensus.state.get_anchor(0);
        assert!(anchor.is_some(), "anchor should be selected for round 0");
    }
}

