//! Ack collection and certificate formation for Shoal consensus.
//!
//! This module handles:
//! - Tracking draft blocks from other validators
//! - Generating acks for valid incoming blocks
//! - Collecting acks for our own blocks
//! - Forming certificates when 2f+1 acks are received

use anyhow::Result;
use modal_common::keypair::Keypair;
use modal_datastore::models::validator::block::Ack;
use modal_datastore::models::ValidatorBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks acks received for blocks in a given round
pub struct AckCollector {
    /// Our peer ID
    pub peer_id: String,
    /// Keypair for signing acks
    pub keypair: Keypair,
    /// Committee size (total number of validators)
    pub committee_size: usize,
    /// Map of (round, peer_id) -> collected acks
    /// Each entry maps the block author's peer_id to the acks we've received for their block
    pub pending_acks: HashMap<(u64, String), Vec<Ack>>,
    /// Blocks we're waiting for acks on (our own blocks)
    pub our_pending_blocks: HashMap<u64, ValidatorBlock>,
    /// Blocks we've received from other validators that need acks
    pub incoming_blocks: HashMap<(u64, String), ValidatorBlock>,
    /// Set of blocks we've already acked (to avoid duplicates)
    pub already_acked: HashMap<(u64, String), bool>,
}

impl AckCollector {
    /// Create a new AckCollector
    pub fn new(peer_id: String, keypair: Keypair, committee_size: usize) -> Self {
        Self {
            peer_id,
            keypair,
            committee_size,
            pending_acks: HashMap::new(),
            our_pending_blocks: HashMap::new(),
            incoming_blocks: HashMap::new(),
            already_acked: HashMap::new(),
        }
    }

    /// Calculate the BFT threshold (2f+1 where f = floor((n-1)/3))
    pub fn threshold(&self) -> usize {
        let f = (self.committee_size - 1) / 3;
        2 * f + 1
    }

    /// Register a block we created and want to collect acks for
    pub fn register_our_block(&mut self, block: ValidatorBlock) {
        let round = block.round_id;
        self.our_pending_blocks.insert(round, block);
    }

    /// Handle an incoming draft block from another validator
    /// Returns an Ack if the block is valid and we should ack it
    pub fn handle_incoming_block(&mut self, block: &ValidatorBlock) -> Result<Option<Ack>> {
        let key = (block.round_id, block.peer_id.clone());

        // Don't ack blocks from ourselves
        if block.peer_id == self.peer_id {
            return Ok(None);
        }

        // Don't ack blocks we've already acked
        if self.already_acked.contains_key(&key) {
            return Ok(None);
        }

        // Validate the block signatures
        if !block.validate_sigs()? {
            log::warn!(
                "Invalid signatures on block from {} round {}",
                &block.peer_id[..16.min(block.peer_id.len())],
                block.round_id
            );
            return Ok(None);
        }

        // Store the block
        self.incoming_blocks.insert(key.clone(), block.clone());

        // Generate and return an ack
        let ack = block.generate_ack(&self.keypair)?;
        
        // Mark as acked
        self.already_acked.insert(key, true);

        Ok(Some(ack))
    }

    /// Handle an incoming ack for one of our blocks
    /// Returns true if we now have enough acks to form a certificate
    pub fn handle_incoming_ack(&mut self, ack: &Ack) -> Result<bool> {
        // Only process acks for blocks we authored
        if ack.peer_id != self.peer_id {
            return Ok(false);
        }

        let key = (ack.round_id, ack.peer_id.clone());

        // Validate the ack signature
        if !validate_ack(ack)? {
            log::warn!(
                "Invalid ack signature from {} for round {}",
                &ack.acker[..16.min(ack.acker.len())],
                ack.round_id
            );
            return Ok(false);
        }

        // Add to pending acks
        let acks = self.pending_acks.entry(key.clone()).or_insert_with(Vec::new);

        // Check if we already have an ack from this validator
        if acks.iter().any(|a| a.acker == ack.acker) {
            return Ok(false);
        }

        acks.push(ack.clone());

        // Check if we have enough acks
        Ok(acks.len() >= self.threshold())
    }

    /// Get our block for a round if it exists
    pub fn get_our_block(&self, round: u64) -> Option<&ValidatorBlock> {
        self.our_pending_blocks.get(&round)
    }

    /// Get the collected acks for a round/peer combination
    pub fn get_acks(&self, round: u64, peer_id: &str) -> Vec<Ack> {
        self.pending_acks
            .get(&(round, peer_id.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// Form a certificate for a block by combining acks
    /// Returns the updated block with certificate attached if successful
    pub fn form_certificate(&mut self, round: u64) -> Option<ValidatorBlock> {
        let key = (round, self.peer_id.clone());
        let acks = self.pending_acks.get(&key)?;

        if acks.len() < self.threshold() {
            return None;
        }

        // Get our block for this round
        let block = self.our_pending_blocks.get_mut(&round)?;

        // Add acks to the block
        for ack in acks {
            block.acks.insert(ack.acker.clone(), ack.acker_sig.clone());
        }

        // Generate certificate by combining ack signatures
        // The certificate is a JSON-encoded list of acker signatures
        let cert_data: Vec<&str> = acks.iter().map(|a| a.acker_sig.as_str()).collect();
        let cert = serde_json::to_string(&cert_data).ok()?;
        block.cert = Some(cert);

        Some(block.clone())
    }

    /// Clean up old entries for completed rounds
    pub fn cleanup_round(&mut self, round: u64) {
        // Remove old pending acks
        self.pending_acks.retain(|(r, _), _| *r > round);
        
        // Remove old incoming blocks
        self.incoming_blocks.retain(|(r, _), _| *r > round);
        
        // Remove old already-acked markers
        self.already_acked.retain(|(r, _), _| *r > round);
        
        // Remove old pending blocks (keep a few rounds for late acks)
        if round > 5 {
            self.our_pending_blocks.retain(|r, _| *r > round - 5);
        }
    }
}

/// Validate an Ack signature
fn validate_ack(ack: &Ack) -> Result<bool> {
    // Create a keypair from the acker's public key for verification
    let acker_keypair = Keypair::from_public_key(&ack.acker, "ed25519")?;
    
    // Reconstruct the facts that were signed
    let facts = serde_json::json!({
        "peer_id": ack.peer_id,
        "round_id": ack.round_id,
        "closing_sig": ack.closing_sig,
    });
    
    // Verify the signature
    acker_keypair.verify_json(&ack.acker_sig, &facts)
}

/// Validate a certificate by checking that it has enough valid ack signatures
pub fn validate_certificate(block: &ValidatorBlock, committee_size: usize) -> Result<bool> {
    // Check that the block has a certificate
    let cert = match &block.cert {
        Some(c) => c,
        None => return Ok(false),
    };
    
    // Calculate threshold
    let f = (committee_size - 1) / 3;
    let threshold = 2 * f + 1;
    
    // Validate that we have enough acks
    if block.acks.len() < threshold {
        log::warn!(
            "Certificate validation failed: {} acks < {} threshold",
            block.acks.len(),
            threshold
        );
        return Ok(false);
    }
    
    // Validate the block's own signatures
    if !block.validate_sigs()? {
        log::warn!("Certificate validation failed: invalid block signatures");
        return Ok(false);
    }
    
    // Validate each ack signature
    let mut valid_acks = 0;
    let closing_sig = block.closing_sig.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Block missing closing signature"))?;
        
    for (acker, acker_sig) in &block.acks {
        let ack = Ack {
            peer_id: block.peer_id.clone(),
            round_id: block.round_id,
            closing_sig: closing_sig.clone(),
            acker: acker.clone(),
            acker_sig: acker_sig.clone(),
        };
        
        match validate_ack(&ack) {
            Ok(true) => valid_acks += 1,
            Ok(false) => log::warn!("Invalid ack from {}", &acker[..16.min(acker.len())]),
            Err(e) => log::warn!("Error validating ack from {}: {}", &acker[..16.min(acker.len())], e),
        }
    }
    
    // Check that we have enough valid acks
    if valid_acks < threshold {
        log::warn!(
            "Certificate validation failed: {} valid acks < {} threshold",
            valid_acks,
            threshold
        );
        return Ok(false);
    }
    
    // Verify the certificate structure matches the acks
    // The cert should be a JSON array of signatures
    if let Ok(cert_sigs) = serde_json::from_str::<Vec<String>>(cert) {
        if cert_sigs.len() != block.acks.len() {
            log::warn!("Certificate signature count mismatch");
            return Ok(false);
        }
    }
    
    Ok(true)
}

/// Save a certified block to the appropriate datastores
pub async fn save_certified_block(
    block: &ValidatorBlock,
    datastore: &Arc<Mutex<DatastoreManager>>,
) -> Result<()> {
    let mgr = datastore.lock().await;
    
    // First save to active store
    block.save_to_active(&mgr).await?;
    
    // If it has a certificate, promote to final
    if block.cert.is_some() {
        block.promote_to_final(&mgr).await?;
        log::info!(
            "✅ Block certified and finalized: round {} peer {}",
            block.round_id,
            &block.peer_id[..16.min(block.peer_id.len())]
        );
    }
    
    Ok(())
}

/// Run periodic finalization task to move certified blocks to final store
pub async fn run_finalization_task(
    datastore: &Arc<Mutex<DatastoreManager>>,
    current_round: u64,
) {
    let mgr = datastore.lock().await;
    
    // Keep blocks in active for 10 rounds before deleting
    match ValidatorBlock::run_finalization(&mgr, current_round, 10).await {
        Ok((finalized, deleted)) => {
            if finalized > 0 || deleted > 0 {
                log::info!(
                    "📁 Finalization: {} blocks promoted, {} blocks cleaned from active store",
                    finalized,
                    deleted
                );
            }
        }
        Err(e) => {
            log::warn!("Finalization task error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_keypair() -> Keypair {
        // Generate a test keypair
        let libp2p_keypair = libp2p_identity::Keypair::generate_ed25519();
        Keypair::from_libp2p_keypair(libp2p_keypair).unwrap()
    }

    fn create_test_block(peer_id: &str, round: u64, keypair: &Keypair) -> ValidatorBlock {
        let mut block = ValidatorBlock {
            peer_id: peer_id.to_string(),
            round_id: round,
            prev_round_certs: HashMap::new(),
            opening_sig: None,
            events: Vec::new(),
            closing_sig: None,
            hash: None,
            acks: HashMap::new(),
            late_acks: Vec::new(),
            cert: None,
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        };
        block.generate_sigs(keypair).unwrap();
        block
    }

    #[test]
    fn test_threshold_calculation() {
        // n=1, f=0, threshold=1
        let collector = AckCollector::new("test".to_string(), create_test_keypair(), 1);
        assert_eq!(collector.threshold(), 1);

        // n=3, f=0, threshold=1
        let collector = AckCollector::new("test".to_string(), create_test_keypair(), 3);
        assert_eq!(collector.threshold(), 1);

        // n=4, f=1, threshold=3
        let collector = AckCollector::new("test".to_string(), create_test_keypair(), 4);
        assert_eq!(collector.threshold(), 3);

        // n=7, f=2, threshold=5
        let collector = AckCollector::new("test".to_string(), create_test_keypair(), 7);
        assert_eq!(collector.threshold(), 5);

        // n=10, f=3, threshold=7
        let collector = AckCollector::new("test".to_string(), create_test_keypair(), 10);
        assert_eq!(collector.threshold(), 7);
    }

    #[test]
    fn test_register_our_block() {
        let keypair = create_test_keypair();
        let peer_id = keypair.as_public_address();
        let mut collector = AckCollector::new(peer_id.clone(), keypair.clone(), 4);

        let block = create_test_block(&peer_id, 1, &keypair);
        collector.register_our_block(block.clone());

        assert!(collector.get_our_block(1).is_some());
        assert!(collector.get_our_block(2).is_none());
    }

    #[test]
    fn test_handle_incoming_block_from_other() {
        let our_keypair = create_test_keypair();
        let our_peer_id = our_keypair.as_public_address();
        let mut collector = AckCollector::new(our_peer_id.clone(), our_keypair.clone(), 4);

        // Create a block from another validator
        let other_keypair = create_test_keypair();
        let other_peer_id = other_keypair.as_public_address();
        let block = create_test_block(&other_peer_id, 1, &other_keypair);

        // Should generate an ack
        let result = collector.handle_incoming_block(&block).unwrap();
        assert!(result.is_some());

        // Second call should return None (already acked)
        let result2 = collector.handle_incoming_block(&block).unwrap();
        assert!(result2.is_none());
    }

    #[test]
    fn test_handle_incoming_block_from_self() {
        let keypair = create_test_keypair();
        let peer_id = keypair.as_public_address();
        let mut collector = AckCollector::new(peer_id.clone(), keypair.clone(), 4);

        // Create a block from ourselves
        let block = create_test_block(&peer_id, 1, &keypair);

        // Should not generate an ack for our own block
        let result = collector.handle_incoming_block(&block).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cleanup_round() {
        let keypair = create_test_keypair();
        let peer_id = keypair.as_public_address();
        let mut collector = AckCollector::new(peer_id.clone(), keypair.clone(), 4);

        // Add some test data
        let block = create_test_block(&peer_id, 5, &keypair);
        collector.register_our_block(block);

        let other_keypair = create_test_keypair();
        let other_peer_id = other_keypair.as_public_address();
        let other_block = create_test_block(&other_peer_id, 5, &other_keypair);
        collector.handle_incoming_block(&other_block).unwrap();

        // Cleanup rounds <= 10 (keeps round 5 due to the 5-round buffer)
        collector.cleanup_round(10);

        // Old entries should be removed
        assert!(collector.incoming_blocks.is_empty());
        assert!(collector.already_acked.is_empty());
    }
}

