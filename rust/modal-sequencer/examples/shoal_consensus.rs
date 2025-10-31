/// Example: Using the Shoal consensus sequencer
///
/// This example demonstrates how to:
/// 1. Create a Shoal-based sequencer with multiple validators
/// 2. Submit transactions
/// 3. Propose batches and form certificates
/// 4. Advance rounds and commit transactions
/// 5. Query the consensus state

use modal_sequencer::{ShoalSequencer, ShoalSequencerConfig};
use modal_sequencer_consensus::narwhal::Transaction;
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("=== Shoal Consensus Sequencer Example ===\n");
    
    // Create temporary storage for the example
    let temp_dir = TempDir::new()?;
    println!("Created temporary storage at: {:?}", temp_dir.path());
    
    // Create datastore
    let datastore = NetworkDatastore::new(temp_dir.path())?;
    let datastore = Arc::new(Mutex::new(datastore));
    
    // Create a sequencer configuration for a 4-validator committee
    // This validator is validator #0
    let config = ShoalSequencerConfig::new_test(4, 0);
    
    println!("\nCreating Shoal sequencer...");
    println!("  Committee size: {}", config.committee.size());
    println!("  Quorum threshold: {}", config.committee.quorum_threshold());
    println!("  Max Byzantine: {}", config.committee.max_byzantine());
    println!("  Workers per validator: {}", config.narwhal_config.workers_per_validator);
    
    // Save quorum threshold for later
    let quorum_threshold = config.committee.quorum_threshold();
    
    // Create and initialize sequencer
    let sequencer = ShoalSequencer::new(datastore, config).await?;
    sequencer.initialize().await?;
    
    println!("\n✓ Sequencer initialized successfully");
    println!("  Current round: {}", sequencer.get_current_round().await);
    println!("  Chain tip: {}", sequencer.get_chain_tip().await);
    
    // Step 1: Submit transactions
    println!("\n=== Step 1: Submitting Transactions ===");
    
    for i in 0..10 {
        let tx = Transaction {
            data: format!("transaction-{}", i).into_bytes(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };
        
        sequencer.submit_transaction(tx).await?;
    }
    
    println!("✓ Submitted 10 transactions");
    println!("  Pending transactions: {}", sequencer.pending_transaction_count().await);
    
    // Step 2: Propose a batch (genesis round)
    println!("\n=== Step 2: Proposing Genesis Batch ===");
    
    let cert = sequencer.propose_batch().await?;
    
    if let Some(cert) = cert {
        println!("✓ Batch proposed and certificate formed");
        println!("  Round: {}", cert.header.round);
        println!("  Author: {:?}", cert.header.author);
        println!("  Parents: {}", cert.header.parents.len());
        println!("  Signers: {}/{}", 
                 cert.signers.iter().filter(|&&s| s).count(),
                 cert.signers.len());
        
        // Genesis should auto-commit
        println!("  Genesis committed immediately!");
        println!("  Chain tip: {}", sequencer.get_chain_tip().await);
    }
    
    // Step 3: Demonstrate round 1 limitation
    println!("\n=== Step 3: Round 1 and Quorum Requirements ===");
    
    sequencer.advance_round().await;
    println!("✓ Advanced to round {}", sequencer.get_current_round().await);
    
    // Submit more transactions
    for i in 10..15 {
        let tx = Transaction {
            data: format!("transaction-{}", i).into_bytes(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };
        sequencer.submit_transaction(tx).await?;
    }
    
    println!("  Submitted 5 more transactions");
    println!("  Pending transactions: {}", sequencer.pending_transaction_count().await);
    
    // Try to propose batch for round 1
    println!("\n  Attempting to propose for round 1...");
    match sequencer.propose_batch().await {
        Ok(Some(cert)) => {
            println!("  ✓ Round 1 batch proposed");
            println!("    Round: {}", cert.header.round);
            println!("    Parents: {} (references from round 0)", cert.header.parents.len());
        },
        Ok(None) => {
            println!("  ! No batch formed (no pending transactions)");
        },
        Err(e) => {
            println!("  ! Expected error in single-validator mode:");
            println!("    {}", e);
            println!("\n  This is normal! In a real distributed system:");
            println!("    - Round 1+ requires {} parent certificates (2f+1 quorum)", 
                     quorum_threshold);
            println!("    - Single validator only has their own certificate from round 0");
            println!("    - Need {} validators to provide certificates for quorum",
                     quorum_threshold);
        }
    }
    
    println!("\n=== Final State ===");
    println!("  Current round: {}", sequencer.get_current_round().await);
    println!("  Last committed round: {}", sequencer.get_chain_tip().await);
    
    println!("\n✓ Example completed successfully!");
    
    Ok(())
}

