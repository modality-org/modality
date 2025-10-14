use modality_network_mining::{Blockchain, ChainConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Modality Network Mining Example ===\n");

    // Define peer IDs for the blockchain
    let genesis_peer_id = "QmGenesis123456789abcdef";
    let nominated_peer_id1 = "QmMiner1aaaabbbbccccdddd";
    let nominated_peer_id2 = "QmMiner2eeeeffffgggghhh";

    // Create a new blockchain with custom configuration
    let config = ChainConfig {
        initial_difficulty: 100, // Low difficulty for demo
        target_block_time_secs: 60, // 1 minute per block
    };

    let mut chain = Blockchain::new(config, genesis_peer_id.to_string());
    println!("✓ Created new blockchain");
    println!("  Genesis block hash: {}", chain.latest_block().header.hash);
    println!("  Genesis peer ID: {}", genesis_peer_id);
    println!("  Height: {}", chain.height());
    println!("  Current epoch: {}\n", chain.current_epoch());

    // Mine a block nominating peer1
    println!("⛏ Mining block 1 (nominating peer1, number: 12345)...");
    let start = std::time::Instant::now();
    let block = chain.mine_block(nominated_peer_id1.to_string(), 12345)?;
    let duration = start.elapsed();

    println!("✓ Block mined successfully!");
    println!("  Hash: {}", block.header.hash);
    println!("  Nonce: {}", block.header.nonce);
    println!("  Difficulty: {}", block.header.difficulty);
    println!("  Miner number: {}", block.data.miner_number);
    println!("  Nominated peer ID: {}", block.data.nominated_peer_id);
    println!("  Time taken: {:?}", duration);
    println!("  Height: {}", chain.height());

    // Mine more blocks to demonstrate epochs
    println!("\n⛏ Mining more blocks with different nominations...");
    for i in 2..=5 {
        let miner_num = 10000 + i;
        let nominated_peer = if i % 2 == 0 { nominated_peer_id1 } else { nominated_peer_id2 };
        let peer_name = if i % 2 == 0 { "peer1" } else { "peer2" };

        let block = chain.mine_block(nominated_peer.to_string(), miner_num)?;
        println!("  Block {}: {} (epoch {}, nominated: {}, number: {})",
            block.header.index,
            &block.header.hash[..16],
            chain.epoch_manager.get_epoch(block.header.index),
            peer_name,
            block.data.miner_number
        );
    }

    // Check nomination statistics
    println!("\n📊 Nomination Statistics:");
    let peer1_count = chain.count_blocks_by_nominated_peer(nominated_peer_id1);
    let peer2_count = chain.count_blocks_by_nominated_peer(nominated_peer_id2);
    let genesis_count = chain.count_blocks_by_nominated_peer(genesis_peer_id);

    println!("  Blocks nominating peer1: {}", peer1_count);
    println!("  Blocks nominating peer2: {}", peer2_count);
    println!("  Blocks nominating genesis peer: {}", genesis_count);

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

    // Get blocks by nominated peer
    println!("\n📦 Blocks nominating peer1:");
    let peer1_blocks = chain.get_blocks_by_nominated_peer(nominated_peer_id1);
    for block in peer1_blocks {
        println!("    Block {}: miner_number = {}",
            block.header.index,
            block.data.miner_number
        );
    }

    println!("\n✓ Example completed successfully!");

    Ok(())
}
