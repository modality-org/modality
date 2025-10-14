use modality_network_mining::{Blockchain, ChainConfig, SigningKey};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Modality Network Mining Example ===\n");

    // Create signing keys
    let genesis_key = SigningKey::from_bytes(&[1u8; 32]);
    let nominated_key1 = SigningKey::from_bytes(&[2u8; 32]);
    let nominated_key2 = SigningKey::from_bytes(&[3u8; 32]);

    // Create a new blockchain with custom configuration
    let config = ChainConfig {
        initial_difficulty: 100, // Low difficulty for demo
        target_block_time_secs: 60, // 1 minute per block
    };

    let mut chain = Blockchain::new(config, genesis_key.verifying_key());
    println!("✓ Created new blockchain");
    println!("  Genesis block hash: {}", chain.latest_block().header.hash);
    println!("  Height: {}", chain.height());
    println!("  Current epoch: {}\n", chain.current_epoch());

    // Mine a block nominating key1
    println!("⛏ Mining block 1 (nominating key1, number: 12345)...");
    let start = std::time::Instant::now();
    let block = chain.mine_block(nominated_key1.verifying_key(), 12345)?;
    let duration = start.elapsed();
    
    println!("✓ Block mined successfully!");
    println!("  Hash: {}", block.header.hash);
    println!("  Nonce: {}", block.header.nonce);
    println!("  Difficulty: {}", block.header.difficulty);
    println!("  Miner number: {}", block.data.miner_number);
    println!("  Nominated public key: {}", hex::encode(block.data.nominated_public_key.to_bytes()));
    println!("  Time taken: {:?}", duration);
    println!("  Height: {}", chain.height());

    // Mine more blocks to demonstrate epochs
    println!("\n⛏ Mining more blocks with different nominations...");
    for i in 2..=5 {
        let miner_num = 10000 + i;
        let nominated_key = if i % 2 == 0 { &nominated_key1 } else { &nominated_key2 };
        let key_name = if i % 2 == 0 { "key1" } else { "key2" };
        
        let block = chain.mine_block(nominated_key.verifying_key(), miner_num)?;
        println!("  Block {}: {} (epoch {}, nominated: {}, number: {})", 
            block.header.index, 
            &block.header.hash[..16], 
            chain.epoch_manager.get_epoch(block.header.index),
            key_name,
            block.data.miner_number
        );
    }

    // Check nomination statistics
    println!("\n📊 Nomination Statistics:");
    let key1_count = chain.count_blocks_by_nominated_key(&nominated_key1.verifying_key());
    let key2_count = chain.count_blocks_by_nominated_key(&nominated_key2.verifying_key());
    let genesis_count = chain.count_blocks_by_nominated_key(&genesis_key.verifying_key());
    
    println!("  Blocks nominating key1: {}", key1_count);
    println!("  Blocks nominating key2: {}", key2_count);
    println!("  Blocks nominating genesis key: {}", genesis_count);

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
        println!("    Block {}: {} (difficulty: {}, miner_number: {})", 
            i, 
            &block.header.hash[..16],
            block.header.difficulty,
            block.data.miner_number
        );
    }

    // Get blocks by nominated key
    println!("\n📦 Blocks nominating key1:");
    let key1_blocks = chain.get_blocks_by_nominated_key(&nominated_key1.verifying_key());
    for block in key1_blocks {
        println!("    Block {}: miner_number = {}", 
            block.header.index,
            block.data.miner_number
        );
    }

    println!("\n✓ Example completed successfully!");

    Ok(())
}
