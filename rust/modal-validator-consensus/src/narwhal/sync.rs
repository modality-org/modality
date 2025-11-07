use crate::narwhal::{Certificate, CertificateDigest, Batch, BatchDigest};
use serde::{Deserialize, Serialize};

/// Request for synchronizing DAG data between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Request certificates by digest
    GetCertificates { digests: Vec<CertificateDigest> },
    
    /// Request all certificates in a specific round
    GetCertificatesInRound { round: u64 },
    
    /// Request certificates in a range of rounds
    GetCertificatesInRange { start_round: u64, end_round: u64 },
    
    /// Request a batch by digest
    GetBatch { digest: BatchDigest },
    
    /// Request multiple batches by digest
    GetBatches { digests: Vec<BatchDigest> },
    
    /// Request the highest round number
    GetHighestRound,
    
    /// Request missing certificates based on parent references
    GetMissingCertificates {
        /// Certificates we have
        known_digests: Vec<CertificateDigest>,
        /// Up to which round to check
        up_to_round: u64,
    },
}

/// Response to a sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    /// Certificates response
    Certificates {
        certificates: Vec<Certificate>,
        /// Optional indicator if more data is available
        has_more: bool,
    },
    
    /// Batch response
    Batches {
        batches: Vec<Batch>,
    },
    
    /// Highest round response
    HighestRound {
        round: u64,
    },
    
    /// Error response
    Error {
        message: String,
    },
    
    /// Empty response (no data found)
    Empty,
}

impl SyncRequest {
    /// Create a request for specific certificates
    pub fn certificates(digests: Vec<CertificateDigest>) -> Self {
        Self::GetCertificates { digests }
    }
    
    /// Create a request for certificates in a round
    pub fn certificates_in_round(round: u64) -> Self {
        Self::GetCertificatesInRound { round }
    }
    
    /// Create a request for certificates in a range
    pub fn certificates_in_range(start_round: u64, end_round: u64) -> Self {
        Self::GetCertificatesInRange { start_round, end_round }
    }
    
    /// Create a request for a batch
    pub fn batch(digest: BatchDigest) -> Self {
        Self::GetBatch { digest }
    }
    
    /// Create a request for multiple batches
    pub fn batches(digests: Vec<BatchDigest>) -> Self {
        Self::GetBatches { digests }
    }
    
    /// Create a request for the highest round
    pub fn highest_round() -> Self {
        Self::GetHighestRound
    }
    
    /// Create a request for missing certificates
    pub fn missing_certificates(known_digests: Vec<CertificateDigest>, up_to_round: u64) -> Self {
        Self::GetMissingCertificates { known_digests, up_to_round }
    }
}

impl SyncResponse {
    /// Create a certificates response
    pub fn certificates(certificates: Vec<Certificate>, has_more: bool) -> Self {
        Self::Certificates { certificates, has_more }
    }
    
    /// Create a batches response
    pub fn batches(batches: Vec<Batch>) -> Self {
        Self::Batches { batches }
    }
    
    /// Create a highest round response
    pub fn highest_round(round: u64) -> Self {
        Self::HighestRound { round }
    }
    
    /// Create an error response
    pub fn error(message: String) -> Self {
        Self::Error { message }
    }
    
    /// Create an empty response
    pub fn empty() -> Self {
        Self::Empty
    }
    
    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
    
    /// Check if this is an empty response
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_request_constructors() {
        let digest = [1u8; 32];
        
        let req = SyncRequest::certificates(vec![digest]);
        assert!(matches!(req, SyncRequest::GetCertificates { .. }));
        
        let req = SyncRequest::certificates_in_round(10);
        assert!(matches!(req, SyncRequest::GetCertificatesInRound { round: 10 }));
        
        let req = SyncRequest::batch(digest);
        assert!(matches!(req, SyncRequest::GetBatch { .. }));
        
        let req = SyncRequest::highest_round();
        assert!(matches!(req, SyncRequest::GetHighestRound));
    }
    
    #[test]
    fn test_sync_response_constructors() {
        let resp = SyncResponse::empty();
        assert!(resp.is_empty());
        
        let resp = SyncResponse::error("test error".to_string());
        assert!(resp.is_error());
        
        let resp = SyncResponse::highest_round(42);
        assert!(matches!(resp, SyncResponse::HighestRound { round: 42 }));
    }
    
    #[test]
    fn test_sync_request_serialization() {
        let req = SyncRequest::certificates_in_range(0, 10);
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: SyncRequest = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            SyncRequest::GetCertificatesInRange { start_round, end_round } => {
                assert_eq!(start_round, 0);
                assert_eq!(end_round, 10);
            }
            _ => panic!("wrong variant"),
        }
    }
}

