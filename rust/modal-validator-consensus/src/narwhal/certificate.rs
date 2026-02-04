use crate::narwhal::{AggregatedSignature, Certificate, Committee, Header, PublicKey, Signature, Vote};
use anyhow::{bail, Result};
use std::collections::HashMap;

/// Builder for creating certificates by collecting votes
pub struct CertificateBuilder {
    header: Header,
    committee: Committee,
    votes: HashMap<PublicKey, Signature>,
}

impl CertificateBuilder {
    /// Create a new certificate builder
    pub fn new(header: Header, committee: Committee) -> Self {
        Self {
            header,
            committee,
            votes: HashMap::new(),
        }
    }

    /// Add a vote from a validator
    pub fn add_vote(&mut self, voter: PublicKey, signature: Signature) -> Result<()> {
        // Verify voter is in committee
        if !self.committee.contains(&voter) {
            bail!("voter not in committee");
        }

        // Check for duplicate vote
        if self.votes.contains_key(&voter) {
            bail!("duplicate vote from {:?}", voter);
        }

        // TODO: Verify signature
        // In a real implementation, we would verify:
        // signature.verify(&voter, header.digest())
        
        self.votes.insert(voter, signature);
        
        Ok(())
    }

    /// Check if we have collected enough votes for quorum
    pub fn has_quorum(&self) -> bool {
        let threshold = self.committee.quorum_threshold();
        self.votes.len() >= threshold as usize
    }

    /// Build the final certificate (requires quorum)
    pub fn build(self) -> Result<Certificate> {
        if !self.has_quorum() {
            bail!(
                "insufficient votes: {} < {}",
                self.votes.len(),
                self.committee.quorum_threshold()
            );
        }

        // Create signers bitmap
        let mut signers = vec![false; self.committee.size()];
        for voter in self.votes.keys() {
            if let Some(index) = self.committee.get_index(voter) {
                signers[index] = true;
            }
        }

        // TODO: Aggregate signatures
        // In a real implementation with BLS, we would:
        // aggregated_sig = aggregate_signatures(self.votes.values())
        let aggregated_signature = AggregatedSignature {
            signature: vec![], // Placeholder
        };

        Ok(Certificate {
            header: self.header,
            aggregated_signature,
            signers,
        })
    }

    /// Get current vote count
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

/// Verify a certificate's validity
pub fn verify_certificate(cert: &Certificate, committee: &Committee) -> Result<()> {
    // Check quorum
    if !cert.has_quorum(committee.size()) {
        bail!("certificate does not have quorum");
    }

    // Verify signers are in committee
    for (idx, &signed) in cert.signers.iter().enumerate() {
        if signed
            && idx >= committee.size() {
                bail!("signer index out of bounds");
            }
            // In real implementation, verify the validator at this index signed
    }

    // TODO: Verify aggregated signature
    // In real implementation:
    // aggregated_signature.verify(&cert.header.digest(), &signers_pubkeys)

    Ok(())
}

/// Create a vote for a header
pub fn create_vote(header: &Header, voter: PublicKey, _private_key: &[u8]) -> Vote {
    // TODO: Actually sign the header with the private key
    // In real implementation:
    // signature = private_key.sign(header.digest())
    
    Vote {
        header_digest: header.digest(),
        round: header.round,
        voter,
        signature: vec![], // Placeholder signature
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
    use crate::narwhal::{Header, Validator};
    use std::net::SocketAddr;

    fn make_test_committee(size: usize) -> Committee {
        let validators: Vec<Validator> = (0..size)
            .map(|i| Validator {
                public_key: vec![i as u8],
                stake: 1,
                network_address: format!("127.0.0.1:800{}", i).parse::<SocketAddr>().unwrap(),
            })
            .collect();
        Committee::new(validators)
    }

    fn make_test_header() -> Header {
        Header {
            author: test_peer_id(0),
            round: 1,
            batch_digest: [0u8; 32],
            parents: vec![],
            timestamp: 1000,
        }
    }

    #[test]
    fn test_certificate_builder_add_vote() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        // Add valid vote
        assert!(builder.add_vote(vec![0], vec![1, 2, 3]).is_ok());
        assert_eq!(builder.vote_count(), 1);
    }

    #[test]
    fn test_certificate_builder_duplicate_vote() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        builder.add_vote(vec![0], vec![1, 2, 3]).unwrap();
        
        // Duplicate vote should fail
        assert!(builder.add_vote(vec![0], vec![4, 5, 6]).is_err());
    }

    #[test]
    fn test_certificate_builder_invalid_voter() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        // Vote from non-committee member
        assert!(builder.add_vote(vec![99], vec![1, 2, 3]).is_err());
    }

    #[test]
    fn test_certificate_builder_quorum() {
        let committee = make_test_committee(4); // Need 3 for quorum
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        assert!(!builder.has_quorum());

        builder.add_vote(vec![0], vec![]).unwrap();
        assert!(!builder.has_quorum());

        builder.add_vote(vec![1], vec![]).unwrap();
        assert!(!builder.has_quorum());

        builder.add_vote(vec![2], vec![]).unwrap();
        assert!(builder.has_quorum());
    }

    #[test]
    fn test_certificate_builder_build() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        // Add 3 votes (quorum)
        builder.add_vote(vec![0], vec![]).unwrap();
        builder.add_vote(vec![1], vec![]).unwrap();
        builder.add_vote(vec![2], vec![]).unwrap();

        let cert = builder.build().unwrap();
        assert_eq!(cert.header.round, header.round);
        assert_eq!(cert.signers, vec![true, true, true, false]);
    }

    #[test]
    fn test_certificate_builder_build_no_quorum() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        // Add only 2 votes (insufficient)
        builder.add_vote(vec![0], vec![]).unwrap();
        builder.add_vote(vec![1], vec![]).unwrap();

        assert!(builder.build().is_err());
    }

    #[test]
    fn test_verify_certificate() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee.clone());

        builder.add_vote(vec![0], vec![]).unwrap();
        builder.add_vote(vec![1], vec![]).unwrap();
        builder.add_vote(vec![2], vec![]).unwrap();

        let cert = builder.build().unwrap();
        assert!(verify_certificate(&cert, &committee).is_ok());
    }
}

