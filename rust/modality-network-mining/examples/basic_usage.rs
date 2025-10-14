use modality_network_mining::{Blockchain, ChainConfig, Transaction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Modality Network Mining Example ===\n");

    // Create a new blockchain with custom configuration
    let config = ChainConfig {
        initial_difficulty: 100, // Low difficulty for demo
        target_block_time_secs: 600, // 10 minutes per block
    };

    let mut chain = Blockchain::new(config);
    println!("✓ Created new blockchain");
    println!("  Genesis block hash: {}", chain.latest_block().header.hash);
    println!("  Height: {}", chain.height());
    println!("  Current epoch: {}\n", chain.current_epoch());

    // Add some transactions
    println!("Adding transactions...");
    for i in 1..=3 {
        let tx = Transaction::new(
            format!("Alice{}", i),
            format!("Bob{}", i),
            100 * i as u64,
            Some(format!("Payment {}", i)),
        );
        chain.add_transaction(tx);
        println!("  + Transaction {}: Alice{} -> Bob{} ({})", i, i, i, 100 * i);
    }

    // Mine the block
    println!("\n⛏ Mining block 1...");
    let start = std::time::Instant::now();
    let block = chain.mine_pending_transactions("Miner1", 50)?;
    let duration = start.elapsed();
    
    println!("✓ Block mined successfully!");
    println!("  Hash: {}", block.header.hash);
    println!("  Nonce: {}", block.header.nonce);
    println!("  Difficulty: {}", block.header.difficulty);
    println!("  Transactions: {}", block.transactions.len());
    println!("  Time taken: {:?}", duration);
    println!("  Height: {}", chain.height());

    // Mine more blocks to demonstrate epoch progression
    println!("\n⛏ Mining more blocks to demonstrate epochs...");
    for _ in 2..=5 {
        chain.add_transaction(Transaction::new(
            "Alice".to_string(),
            "Bob".to_string(),
            50,
            None,
        ));
        
        let block = chain.mine_pending_transactions("Miner1", 50)?;
        println!("  Block {}: {} (epoch {})", 
            block.header.index, 
            &block.header.hash[..16], 
            chain.epoch_manager.get_epoch(block.header.index)
        );
    }

    // Check balances
    println!("\n💰 Balances:");
    println!("  Miner1: {}", chain.get_balance("Miner1"));
    println!("  Alice1: {}", chain.get_balance("Alice1"));
    println!("  Bob1: {}", chain.get_balance("Bob1"));
    println!("  Bob2: {}", chain.get_balance("Bob2"));
    println!("  Bob3: {}", chain.get_balance("Bob3"));

    // Validate the entire chain
    println!("\n✓ Validating blockchain...");
    chain.validate_chain()?;
    println!("  Chain is valid!");

    // Chain statistics
    println!("\n📊 Blockchain Statistics:");
    println!("  Total blocks: {}", chain.blocks.len());
    println!("  Current height: {}", chain.height());
    println!("  Current epoch: {}", chain.current_epoch());
    println!("  Blocks per epoch: {}", chain.epoch_manager.blocks_per_epoch);
    println!("  Current difficulty: {}", chain.latest_block().header.difficulty);

    // Get epoch blocks
    let epoch_0_blocks = chain.get_epoch_blocks(0);
    println!("\n  Epoch 0 blocks: {}", epoch_0_blocks.len());
    for (i, block) in epoch_0_blocks.iter().enumerate() {
        println!("    Block {}: {} (difficulty: {})", 
            i, 
            &block.header.hash[..16],
            block.header.difficulty
        );
    }

    println!("\n✓ Example completed successfully!");

    Ok(())
}

