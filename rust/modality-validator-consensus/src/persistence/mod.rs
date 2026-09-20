pub mod recovery;

use crate::narwhal::{
    AggregatedSignature, Batch, BatchDigest, Certificate, CertificateDigest,
    Header, PublicKey, Transaction,
};
use anyhow::{Context, Result};
use libp2p_identity::PeerId;
use modal_datastore::models::{
    DAGBatch, DAGCertificate, ConsensusMetadata, DAGState,
};
use std::str::FromStr;

/// Convert a digest (32-byte array) to hex string
pub fn digest_to_hex(digest: &[u8; 32]) -> String {
    hex::encode(digest)
}

/// Convert hex string to digest (32-byte array)
pub fn hex_to_digest(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(hex_str).context("invalid hex string")?;
    if bytes.len() != 32 {
        anyhow::bail!("digest must be 32 bytes, got {}", bytes.len());
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes);
    Ok(digest)
}

/// Convert PeerId to string representation
pub fn peer_id_to_string(peer_id: &PeerId) -> String {
    peer_id.to_base58()
}

/// Convert string to PeerId
pub fn string_to_peer_id(s: &str) -> Result<PeerId> {
    PeerId::from_str(s).context("invalid peer id")
}

/// Trait for converting consensus types to persistence models
pub trait ToPersistenceModel<T> {
    fn to_persistence_model(&self) -> Result<T>;
}

/// Trait for converting persistence models to consensus types
pub trait FromPersistenceModel<T> {
    fn from_persistence_model(model: &T) -> Result<Self>
    where
        Self: Sized;
}

// Certificate conversions
impl ToPersistenceModel<DAGCertificate> for Certificate {
    fn to_persistence_model(&self) -> Result<DAGCertificate> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(DAGCertificate {
            digest: digest_to_hex(&self.digest()),
            author: peer_id_to_string(&self.header.author),
            round: self.header.round,
            header: serde_json::to_string(&self.header)?,
            aggregated_signature: serde_json::to_string(&self.aggregated_signature)?,
            signers: self.signers.clone(),
            batch_digest: digest_to_hex(&self.header.batch_digest),
            parents: self.header.parents.iter().map(digest_to_hex).collect(),
            timestamp: self.header.timestamp,
            committed: false,
            committed_at_round: None,
            created_at: now,
        })
    }
}

impl FromPersistenceModel<DAGCertificate> for Certificate {
    fn from_persistence_model(model: &DAGCertificate) -> Result<Self> {
        let header: Header = serde_json::from_str(&model.header)?;
        let aggregated_signature: AggregatedSignature = 
            serde_json::from_str(&model.aggregated_signature)?;
        
        Ok(Certificate {
            header,
            aggregated_signature,
            signers: model.signers.clone(),
        })
    }
}

// Batch conversions
impl ToPersistenceModel<DAGBatch> for Batch {
    fn to_persistence_model(&self) -> Result<DAGBatch> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let transactions_json = serde_json::to_string(&self.transactions)?;
        let size_bytes = transactions_json.len();

        Ok(DAGBatch {
            digest: digest_to_hex(&self.digest()),
            worker_id: self.worker_id,
            author: String::new(), // Will be set by caller who knows the validator
            transactions: transactions_json,
            transaction_count: self.transactions.len(),
            timestamp: self.timestamp,
            size_bytes,
            referenced_by_cert: None,
            created_at: now,
        })
    }
}

impl FromPersistenceModel<DAGBatch> for Batch {
    fn from_persistence_model(model: &DAGBatch) -> Result<Self> {
        let transactions: Vec<Transaction> = serde_json::from_str(&model.transactions)?;
        
        Ok(Batch {
            transactions,
            worker_id: model.worker_id,
            timestamp: model.timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digest_conversion() {
        let digest = [42u8; 32];
        let hex = digest_to_hex(&digest);
        assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
        
        let back = hex_to_digest(&hex).unwrap();
        assert_eq!(back, digest);
    }

    #[test]
    fn test_peer_id_conversion() {
        use libp2p_identity::ed25519;
        
        let keypair = ed25519::Keypair::generate();
        let peer_id = PeerId::from_public_key(&keypair.public().into());
        
        let s = peer_id_to_string(&peer_id);
        let back = string_to_peer_id(&s).unwrap();
        assert_eq!(back, peer_id);
    }

    #[test]
    fn test_certificate_roundtrip() {
        use libp2p_identity::ed25519;
        
        let keypair = ed25519::Keypair::generate();
        let peer_id = PeerId::from_public_key(&keypair.public().into());
        
        let header = Header {
            author: peer_id,
            round: 1,
            batch_digest: [1u8; 32],
            parents: vec![[2u8; 32], [3u8; 32]],
            timestamp: 1000,
        };
        
        let cert = Certificate {
            header,
            aggregated_signature: AggregatedSignature {
                signature: vec![1, 2, 3],
            },
            signers: vec![true, false, true],
        };
        
        let model = cert.to_persistence_model().unwrap();
        assert_eq!(model.round, 1);
        assert_eq!(model.signers.len(), 3);
        
        let back = Certificate::from_persistence_model(&model).unwrap();
        assert_eq!(back.header.round, cert.header.round);
        assert_eq!(back.signers, cert.signers);
    }

    #[test]
    fn test_batch_roundtrip() {
        let batch = Batch {
            transactions: vec![
                Transaction { data: vec![1, 2, 3], timestamp: 100 },
                Transaction { data: vec![4, 5, 6], timestamp: 200 },
            ],
            worker_id: 1,
            timestamp: 1000,
        };
        
        let model = batch.to_persistence_model().unwrap();
        assert_eq!(model.worker_id, 1);
        assert_eq!(model.transaction_count, 2);
        
        let back = Batch::from_persistence_model(&model).unwrap();
        assert_eq!(back.transactions.len(), batch.transactions.len());
        assert_eq!(back.worker_id, batch.worker_id);
    }
}

