use crate::narwhal::{Certificate, CertificateDigest, SyncRequest, SyncResponse};
use crate::narwhal::dag::DAG;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Client for synchronizing DAG data with remote nodes
pub struct SyncClient {
    /// Local DAG
    dag: Arc<RwLock<DAG>>,
}

impl SyncClient {
    /// Create a new sync client
    pub fn new(dag: Arc<RwLock<DAG>>) -> Self {
        Self { dag }
    }
    
    /// Sync with a remote node by requesting missing certificates
    pub async fn sync_with_peer<F, Fut>(
        &self,
        request_fn: F,
    ) -> Result<SyncStats>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = Result<SyncResponse>>,
    {
        let mut stats = SyncStats::default();
        
        // Get our highest round
        let our_highest_round = {
            let dag = self.dag.read().await;
            dag.highest_round()
        };
        
        // Request peer's highest round
        let peer_highest_round = match request_fn(SyncRequest::highest_round()).await? {
            SyncResponse::HighestRound { round } => round,
            _ => anyhow::bail!("unexpected response to GetHighestRound"),
        };
        
        log::debug!("Our round: {}, peer round: {}", our_highest_round, peer_highest_round);
        
        // Always sync from round 0 to peer's highest to ensure we get all certificates
        // The DAG will skip duplicates during insertion
        let batch_size = 10; // Request 10 rounds at a time
        let mut start_round = 0;
        
        while start_round <= peer_highest_round {
            let end_round = (start_round + batch_size - 1).min(peer_highest_round);
            
            log::debug!("Requesting certificates for rounds {} to {}", start_round, end_round);
            
            let response = request_fn(SyncRequest::certificates_in_range(start_round, end_round)).await?;
            
            match response {
                SyncResponse::Certificates { certificates, has_more } => {
                    log::info!("Received {} certificates from peer", certificates.len());
                    
                    // Insert certificates into our DAG
                    let mut dag = self.dag.write().await;
                    for cert in certificates {
                        match dag.insert(cert) {
                            Ok(()) => stats.certificates_synced += 1,
                            Err(e) => {
                                // Ignore duplicate certificate errors
                                if e.to_string().contains("already exists") {
                                    continue;
                                }
                                log::warn!("Failed to insert certificate: {}", e);
                                stats.certificates_failed += 1;
                            }
                        }
                    }
                    
                    if !has_more {
                        start_round = end_round + 1;
                    }
                }
                SyncResponse::Empty => {
                    start_round = end_round + 1;
                }
                SyncResponse::Error { message } => {
                    anyhow::bail!("sync error: {}", message);
                }
                _ => anyhow::bail!("unexpected response to GetCertificatesInRange"),
            }
        }
        
        Ok(stats)
    }
    
    /// Request specific certificates by digest
    pub async fn request_certificates<F, Fut>(
        &self,
        digests: Vec<CertificateDigest>,
        request_fn: F,
    ) -> Result<Vec<Certificate>>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = Result<SyncResponse>>,
    {
        if digests.is_empty() {
            return Ok(Vec::new());
        }
        
        let response = request_fn(SyncRequest::certificates(digests)).await?;
        
        match response {
            SyncResponse::Certificates { certificates, .. } => Ok(certificates),
            SyncResponse::Empty => Ok(Vec::new()),
            SyncResponse::Error { message } => {
                anyhow::bail!("certificate request failed: {}", message)
            }
            _ => anyhow::bail!("unexpected response to GetCertificates"),
        }
    }
    
    /// Sync missing parents for a certificate
    /// Returns true if all parents were successfully synced
    pub async fn sync_missing_parents<F, Fut>(
        &self,
        cert: &Certificate,
        request_fn: F,
    ) -> Result<bool>
    where
        F: Fn(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = Result<SyncResponse>>,
    {
        let missing_parents = {
            let dag = self.dag.read().await;
            dag.get_missing_parents(cert)
        };
        
        if missing_parents.is_empty() {
            return Ok(true);
        }
        
        log::debug!("Syncing {} missing parents", missing_parents.len());
        
        let certificates = self.request_certificates(missing_parents, request_fn).await?;
        
        // Insert parents into DAG
        let mut dag = self.dag.write().await;
        for parent in certificates {
            dag.insert(parent).context("failed to insert parent certificate")?;
        }
        
        // Check if we now have all parents
        Ok(dag.has_all_parents(cert))
    }
    
    /// Detect and sync any gaps in our DAG
    pub async fn sync_gaps<F, Fut>(
        &self,
        up_to_round: u64,
        request_fn: F,
    ) -> Result<SyncStats>
    where
        F: Fn(SyncRequest) -> Fut + Copy,
        Fut: std::future::Future<Output = Result<SyncResponse>>,
    {
        let mut stats = SyncStats::default();
        
        // Find missing certificates
        let missing = {
            let dag = self.dag.read().await;
            dag.get_missing_certificates_up_to_round(up_to_round)
        };
        
        if missing.is_empty() {
            return Ok(stats);
        }
        
        log::info!("Found {} missing certificates, requesting from peer", missing.len());
        
        // Request missing certificates in batches
        let batch_size = 100;
        for chunk in missing.chunks(batch_size) {
            let certificates = self.request_certificates(chunk.to_vec(), request_fn).await?;
            
            let mut dag = self.dag.write().await;
            for cert in certificates {
                match dag.insert(cert) {
                    Ok(()) => stats.certificates_synced += 1,
                    Err(e) => {
                        log::warn!("Failed to insert missing certificate: {}", e);
                        stats.certificates_failed += 1;
                    }
                }
            }
        }
        
        Ok(stats)
    }
}

/// Statistics from a sync operation
#[derive(Debug, Default, Clone)]
pub struct SyncStats {
    pub certificates_synced: usize,
    pub certificates_failed: usize,
}

impl SyncStats {
    pub fn total_attempted(&self) -> usize {
        self.certificates_synced + self.certificates_failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narwhal::{Header, AggregatedSignature};
    use libp2p_identity::{ed25519, PeerId};
    
    fn test_peer_id(seed: u8) -> PeerId {
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes)
            .expect("valid secret key");
        let keypair = ed25519::Keypair::from(secret);
        PeerId::from_public_key(&keypair.public().into())
    }
    
    fn make_test_cert(author_seed: u8, round: u64, parents: Vec<[u8; 32]>) -> Certificate {
        Certificate {
            header: Header {
                author: test_peer_id(author_seed),
                round,
                batch_digest: [round as u8; 32],
                parents,
                timestamp: 1000 + round,
            },
            aggregated_signature: AggregatedSignature { signature: vec![1, 2, 3] },
            signers: vec![true, true, true],
        }
    }
    
    #[tokio::test]
    async fn test_sync_client_request_certificates() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let client = SyncClient::new(dag);
        
        let cert1 = make_test_cert(1, 0, vec![]);
        let digest1 = cert1.digest();
        
        // Mock request function
        let request_fn = |req: SyncRequest| async move {
            match req {
                SyncRequest::GetCertificates { .. } => {
                    Ok(SyncResponse::Certificates {
                        certificates: vec![cert1.clone()],
                        has_more: false,
                    })
                }
                _ => Ok(SyncResponse::Empty),
            }
        };
        
        let certs = client.request_certificates(vec![digest1], request_fn).await.unwrap();
        assert_eq!(certs.len(), 1);
    }
    
    #[tokio::test]
    async fn test_sync_missing_parents() {
        let dag = Arc::new(RwLock::new(DAG::new()));
        let client = SyncClient::new(dag.clone());
        
        let cert0 = make_test_cert(1, 0, vec![]);
        let digest0 = cert0.digest();
        
        let cert1 = make_test_cert(2, 1, vec![digest0]);
        
        // Mock request function that returns parent
        let request_fn = |req: SyncRequest| async move {
            match req {
                SyncRequest::GetCertificates { .. } => {
                    Ok(SyncResponse::Certificates {
                        certificates: vec![cert0.clone()],
                        has_more: false,
                    })
                }
                _ => Ok(SyncResponse::Empty),
            }
        };
        
        // Sync missing parents
        let success = client.sync_missing_parents(&cert1, request_fn).await.unwrap();
        assert!(success);
        
        // Verify parent is now in DAG
        let dag_read = dag.read().await;
        assert!(dag_read.get(&digest0).is_some());
    }
}

