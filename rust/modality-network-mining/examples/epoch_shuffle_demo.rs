use modality_network_mining::{Blockchain, ChainConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Epoch Nomination Shuffling Demo ===\n");

    // Define genesis peer ID
    let genesis_peer_id = "QmGenesisAbcd1234567890";

    // Create a blockchain with low difficulty for fast mining
    let config = ChainConfig {
        initial_difficulty: 50,
        target_block_time_secs: 60,
    };

    let mut chain = Blockchain::new(config, genesis_peer_id.to_string());
    println!("✓ Created blockchain with genesis block");
    println!("  Genesis peer ID: {}", genesis_peer_id);
    println!("  Blocks per epoch: {}\n", chain.epoch_manager.blocks_per_epoch);

    // Create different peer IDs to nominate
    println!("Creating 10 different peer IDs for nominations...");
    let peer_ids: Vec<String> = (0..10)
        .map(|i| format!("QmMiner{}xxyyzz{:08x}", i, i * 12345))
        .collect();

    // Mine 39 blocks to complete epoch 0 (genesis + 39 = 40 total)
    println!("⛏ Mining 39 blocks to complete epoch 0...\n");
    
    for i in 0..39 {
        // Cycle through peer IDs for nominations
        let nominated_peer_id = &peer_ids[i % peer_ids.len()];
        let miner_number = 1000 + i as u64;
        
        let block = chain.mine_block(nominated_peer_id.clone(), miner_number)?;
        
        if i < 5 || i >= 34 {
            println!("  Block {}: nominated peer {}, number {}, nonce {}",
                block.header.index,
                i % peer_ids.len(),
                miner_number,
                block.header.nonce
            );
        } else if i == 5 {
            println!("  ... mining blocks 6-34 ...");
        }
    }

    println!("\n✓ Epoch 0 complete with 40 blocks!\n");

    // Check if we can get shuffled nominations
    println!("📊 Analyzing epoch 0...\n");

    // Show original order
    println!("Original nomination order (first 10 blocks):");
    let epoch_blocks = chain.get_epoch_blocks(0);
    for (i, block) in epoch_blocks.iter().take(10).enumerate() {
        let peer_id = &block.data.nominated_peer_id;
        println!("  Block {}: peer {} (number {})",
            i, &peer_id[..20], block.data.miner_number);
    }

    // Calculate and show the epoch seed
    let epoch_blocks_owned: Vec<_> = epoch_blocks.iter().map(|b| (*b).clone()).collect();
    let seed = chain.epoch_manager.calculate_epoch_seed(&epoch_blocks_owned);
    println!("\n🎲 Epoch seed (XOR of all nonces): {}", seed);
    println!("   (This seed is derived from XORing all 40 nonces together)\n");

    // Get shuffled nominations
    match chain.get_epoch_shuffled_nominations(0) {
        Some(shuffled) => {
            println!("🔀 Shuffled nominations (first 10 of 40):");
            for (i, (original_idx, peer_id)) in shuffled.iter().take(10).enumerate() {
                let block = &epoch_blocks[*original_idx];
                println!("  Position {}: Block {} (peer {}..., number {})",
                    i, original_idx, &peer_id[..20], block.data.miner_number);
            }

            println!("\n  ... and 30 more shuffled nominations ...");

            // Show last 5
            println!("\n🔀 Last 5 shuffled nominations:");
            for (i, (original_idx, peer_id)) in shuffled.iter().skip(35).enumerate() {
                let block = &epoch_blocks[*original_idx];
                println!("  Position {}: Block {} (peer {}..., number {})",
                    35 + i, original_idx, &peer_id[..20], block.data.miner_number);
            }

            // Verify determinism
            println!("\n✓ Verifying deterministic shuffle...");
            let shuffled2 = chain.get_epoch_shuffled_nominations(0).unwrap();
            if shuffled == shuffled2 {
                println!("  ✓ Shuffle is deterministic (same result on repeated calls)");
            }

            // Get just the peer IDs
            let shuffled_peer_ids = chain.get_epoch_shuffled_peer_ids(0).unwrap();
            println!("\n📋 Shuffled peer IDs can be used for:");
            println!("  - Validator selection");
            println!("  - Consensus participation");
            println!("  - Reward distribution");
            println!("  - Governance voting order");
            println!("\n  Total shuffled peer IDs: {}", shuffled_peer_ids.len());
        }
        None => {
            println!("⚠ Epoch 0 is not complete yet");
        }
    }

    // Try incomplete epoch
    println!("\n📊 Checking epoch 1 (incomplete)...");
    match chain.get_epoch_shuffled_nominations(1) {
        Some(_) => println!("  Epoch 1 is complete"),
        None => println!("  ⚠ Epoch 1 is incomplete (no shuffled nominations available)"),
    }

    println!("\n✓ Demo completed successfully!");
    println!("\nKey takeaways:");
    println!("  • Nonces from all blocks in an epoch are XORed to create a seed");
    println!("  • The seed determines a deterministic shuffle via Fisher-Yates");
    println!("  • Shuffled nominations are only available for complete epochs (40 blocks)");
    println!("  • The shuffle is deterministic: same blocks = same shuffle");

    Ok(())
}

