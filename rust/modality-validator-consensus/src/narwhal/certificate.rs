use crate::narwhal::{AggregatedSignature, Certificate, Committee, Header, PublicKey, Signature, Vote};
use anyhow::{bail, Result};
use libp2p_identity::{Keypair, PublicKey as Libp2pPublicKey};
use std::collections::HashMap;

/// Ed25519 signature length used when concatenating votes into a certificate.
pub const ED25519_SIGNATURE_LENGTH: usize = 64;

/// Extract a libp2p public key from a PeerId (identity-encoded protobuf).
pub fn public_key_from_peer_id(peer_id: &PublicKey) -> Result<Libp2pPublicKey> {
    Libp2pPublicKey::try_decode_protobuf(peer_id.as_ref().digest())
        .map_err(|e| anyhow::anyhow!("peer id does not embed a public key: {e}"))
}

/// Verify that `signature` is a valid signature of `digest` by `voter`.
pub fn verify_signature(voter: &PublicKey, digest: &[u8], signature: &[u8]) -> Result<()> {
    if signature.is_empty() {
        bail!("empty signature");
    }
    let pk = public_key_from_peer_id(voter)?;
    if !pk.verify(digest, signature) {
        bail!("invalid signature from {voter}");
    }
    Ok(())
}

/// Verify a vote against its claimed header digest and voter.
pub fn verify_vote(vote: &Vote) -> Result<()> {
    verify_signature(&vote.voter, &vote.header_digest, &vote.signature)
}

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

    /// Add a vote from a validator, verifying the signature over the header digest.
    pub fn add_vote(&mut self, voter: PublicKey, signature: Signature) -> Result<()> {
        if !self.committee.contains(&voter) {
            bail!("voter not in committee");
        }

        if self.votes.contains_key(&voter) {
            bail!("duplicate vote from {:?}", voter);
        }

        let digest = self.header.digest();
        verify_signature(&voter, &digest, &signature)?;

        self.votes.insert(voter, signature);

        Ok(())
    }

    /// Check if we have collected enough votes for quorum
    pub fn has_quorum(&self) -> bool {
        let threshold = self.committee.quorum_threshold();
        self.votes.len() >= threshold as usize
    }

    /// Get current vote count
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Build the final certificate (requires quorum).
    ///
    /// Signatures are concatenated in committee order for the signers bitmap
    /// so `verify_certificate` can check each vote independently.
    pub fn build(self) -> Result<Certificate> {
        if !self.has_quorum() {
            bail!(
                "insufficient votes: {} < {}",
                self.votes.len(),
                self.committee.quorum_threshold()
            );
        }

        let mut signers = vec![false; self.committee.size()];
        let mut aggregated = Vec::new();
        for (idx, voter) in self.committee.validator_order.iter().enumerate() {
            if let Some(sig) = self.votes.get(voter) {
                signers[idx] = true;
                aggregated.extend_from_slice(sig);
            }
        }

        Ok(Certificate {
            header: self.header,
            aggregated_signature: AggregatedSignature {
                signature: aggregated,
            },
            signers,
        })
    }
}

/// Verify a certificate's quorum and every embedded vote signature.
pub fn verify_certificate(cert: &Certificate, committee: &Committee) -> Result<()> {
    if !cert.has_quorum(committee.size()) {
        bail!("certificate does not have quorum");
    }

    if cert.signers.len() != committee.size() {
        bail!(
            "signer bitmap length {} does not match committee size {}",
            cert.signers.len(),
            committee.size()
        );
    }

    let digest = cert.header.digest();
    let sigs = &cert.aggregated_signature.signature;
    let mut offset = 0;

    for (idx, &signed) in cert.signers.iter().enumerate() {
        if !signed {
            continue;
        }
        let voter = committee
            .validator_order
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("signer index {idx} out of bounds"))?;

        if offset + ED25519_SIGNATURE_LENGTH > sigs.len() {
            bail!("truncated aggregated signature at signer index {idx}");
        }
        let sig = &sigs[offset..offset + ED25519_SIGNATURE_LENGTH];
        offset += ED25519_SIGNATURE_LENGTH;
        verify_signature(voter, &digest, sig)?;
    }

    if offset != sigs.len() {
        bail!(
            "aggregated signature length mismatch: consumed {offset}, have {}",
            sigs.len()
        );
    }

    Ok(())
}

/// Create a vote for a header using the voter's libp2p keypair.
pub fn create_vote(header: &Header, keypair: &Keypair) -> Result<Vote> {
    let header_digest = header.digest();
    let voter = keypair.public().to_peer_id();
    let signature = keypair
        .sign(&header_digest)
        .map_err(|e| anyhow::anyhow!("failed to sign header: {e}"))?;
    Ok(Vote {
        header_digest,
        round: header.round,
        voter,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narwhal::{Header, Validator};
    use libp2p_identity::ed25519;
    use std::net::SocketAddr;

    fn test_libp2p_keypair(seed: u8) -> Keypair {
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = seed;
        let secret = ed25519::SecretKey::try_from_bytes(secret_bytes).expect("valid secret key");
        Keypair::from(ed25519::Keypair::from(secret))
    }

    fn test_peer_id(seed: u8) -> libp2p_identity::PeerId {
        test_libp2p_keypair(seed).public().to_peer_id()
    }

    fn sign_header(seed: u8, header: &Header) -> Signature {
        create_vote(header, &test_libp2p_keypair(seed))
            .expect("sign")
            .signature
    }

    fn make_test_committee(size: usize) -> Committee {
        let validators: Vec<Validator> = (0..size)
            .map(|i| Validator {
                public_key: test_peer_id(i as u8),
                stake: 1,
                network_address: format!("127.0.0.1:800{i}")
                    .parse::<SocketAddr>()
                    .unwrap(),
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
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        assert!(builder.add_vote(test_peer_id(0), sign_header(0, &header)).is_ok());
        assert_eq!(builder.vote_count(), 1);
    }

    #[test]
    fn test_certificate_builder_duplicate_vote() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .unwrap();

        assert!(builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .is_err());
    }

    #[test]
    fn test_certificate_builder_invalid_voter() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        assert!(builder
            .add_vote(test_peer_id(99), sign_header(99, &header))
            .is_err());
    }

    #[test]
    fn test_certificate_builder_rejects_bad_signature() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header, committee);

        assert!(builder.add_vote(test_peer_id(0), vec![0u8; 64]).is_err());
    }

    #[test]
    fn test_certificate_builder_quorum() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        assert!(!builder.has_quorum());

        builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .unwrap();
        assert!(!builder.has_quorum());

        builder
            .add_vote(test_peer_id(1), sign_header(1, &header))
            .unwrap();
        assert!(!builder.has_quorum());

        builder
            .add_vote(test_peer_id(2), sign_header(2, &header))
            .unwrap();
        assert!(builder.has_quorum());
    }

    #[test]
    fn test_certificate_builder_build() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .unwrap();
        builder
            .add_vote(test_peer_id(1), sign_header(1, &header))
            .unwrap();
        builder
            .add_vote(test_peer_id(2), sign_header(2, &header))
            .unwrap();

        let cert = builder.build().unwrap();
        assert_eq!(cert.header.round, header.round);
        assert_eq!(cert.signers, vec![true, true, true, false]);
        assert_eq!(
            cert.aggregated_signature.signature.len(),
            3 * ED25519_SIGNATURE_LENGTH
        );
    }

    #[test]
    fn test_certificate_builder_build_no_quorum() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee);

        builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .unwrap();
        builder
            .add_vote(test_peer_id(1), sign_header(1, &header))
            .unwrap();

        assert!(builder.build().is_err());
    }

    #[test]
    fn test_verify_certificate() {
        let committee = make_test_committee(4);
        let header = make_test_header();
        let mut builder = CertificateBuilder::new(header.clone(), committee.clone());

        builder
            .add_vote(test_peer_id(0), sign_header(0, &header))
            .unwrap();
        builder
            .add_vote(test_peer_id(1), sign_header(1, &header))
            .unwrap();
        builder
            .add_vote(test_peer_id(2), sign_header(2, &header))
            .unwrap();

        let cert = builder.build().unwrap();
        assert!(verify_certificate(&cert, &committee).is_ok());
    }

    #[test]
    fn test_create_vote_matches_peer_id() {
        let header = make_test_header();
        let kp = test_libp2p_keypair(0);
        let vote = create_vote(&header, &kp).unwrap();
        assert_eq!(vote.voter, kp.public().to_peer_id());
        assert_eq!(vote.header_digest, header.digest());
        assert!(verify_vote(&vote).is_ok());
    }
}
