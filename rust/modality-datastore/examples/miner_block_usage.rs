use modality_datastore::{models::MinerBlock, DatastoreManager};

/// The epoch the example's chain has reached (blocks 40–42 are in epoch 1).
const CURRENT_EPOCH: u64 = 1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MinerBlock Datastore Example ===\n");

    // Create an in-memory datastore for demonstration
    let datastore = DatastoreManager::create_in_memory()?;
    println!("✓ Created in-memory datastore\n");

    // Create and save some canonical blocks in epoch 0
    println!("Creating canonical blocks in epoch 0...");
    for i in 1..=5 {
        let block = MinerBlock::new_canonical(
            format!("block_hash_{}", i),
            i,
            0, // epoch 0
            1234567890 + i as i64,
            format!("prev_hash_{}", i - 1),
            format!("data_hash_{}", i),
            10000 + i as u128,
            1000,
            format!("peer_id_{}", i),
            100 + i,
        );

        block.save_to_active(&datastore).await?;
        println!("  Saved block {} (hash: {})", i, block.hash);
    }

    // Create and save an orphaned block
    println!("\nCreating an orphaned block at index 3...");
    let orphaned = MinerBlock::new_orphaned(
        "orphaned_block_hash_3".to_string(),
        3,
        0,
        1234567893,
        "prev_hash_2".to_string(),
        "orphaned_data_hash".to_string(),
        99999,
        1000,
        "orphaned_peer_id".to_string(),
        999,
        "Chain reorganization - longer chain found".to_string(),
        Some("block_hash_3".to_string()),
    );

    orphaned.save_to_active(&datastore).await?;
    println!("  Saved orphaned block (hash: {})", orphaned.hash);
    println!(
        "  Orphan reason: {}",
        orphaned.orphan_reason.as_ref().unwrap()
    );

    // Create more blocks in epoch 1
    println!("\nCreating blocks in epoch 1...");
    for i in 40..=42 {
        let block = MinerBlock::new_canonical(
            format!("block_hash_{}", i),
            i,
            1, // epoch 1
            1234567890 + i as i64,
            format!("prev_hash_{}", i - 1),
            format!("data_hash_{}", i),
            10000 + i as u128,
            1000,
            format!("peer_id_{}", i),
            100 + i,
        );

        block.save_to_active(&datastore).await?;
        println!("  Saved block {} (hash: {})", i, block.hash);
    }

    // Query by hash
    println!("\n📊 Querying blocks...\n");

    if let Some(block) = MinerBlock::find_by_hash_multi(&datastore, "block_hash_3").await? {
        println!("Found block by hash 'block_hash_3':");
        println!("  Index: {}", block.index);
        println!("  Epoch: {}", block.epoch);
        println!("  Is canonical: {}", block.is_canonical);
        println!("  Miner number: {}", block.miner_number);
    }

    // Find all canonical blocks in epoch 0
    println!("\nFinding all canonical blocks in epoch 0:");
    let epoch_0_blocks =
        MinerBlock::find_canonical_by_epoch_multi(&datastore, 0, CURRENT_EPOCH).await?;
    println!("  Found {} canonical blocks", epoch_0_blocks.len());
    for block in &epoch_0_blocks {
        println!(
            "    Block {} (hash: {}, miner_number: {})",
            block.index, block.hash, block.miner_number
        );
    }

    // Find all orphaned blocks
    println!("\nFinding all orphaned blocks:");
    let orphaned_blocks = MinerBlock::find_all_orphaned_multi(&datastore).await?;
    println!("  Found {} orphaned blocks", orphaned_blocks.len());
    for block in &orphaned_blocks {
        println!("    Block {} (hash: {})", block.index, block.hash);
        println!(
            "      Reason: {}",
            block.orphan_reason.as_ref().unwrap_or(&"N/A".to_string())
        );
        if let Some(competing) = &block.competing_hash {
            println!("      Competing hash: {}", competing);
        }
    }

    // Find blocks by index (may include both canonical and orphaned)
    println!("\nFinding all blocks at index 3:");
    let index_3_blocks = MinerBlock::find_by_index_multi(&datastore, 3).await?;
    println!("  Found {} blocks at index 3", index_3_blocks.len());
    for block in &index_3_blocks {
        println!(
            "    Hash: {}, Is canonical: {}, Is orphaned: {}",
            block.hash, block.is_canonical, block.is_orphaned
        );
    }

    // Find canonical block at index
    println!("\nFinding canonical block at index 3:");
    if let Some(block) =
        MinerBlock::find_canonical_by_index_multi(&datastore, 3, CURRENT_EPOCH).await?
    {
        println!("  Found canonical block: {}", block.hash);
        println!("  Nominated peer ID: {}", block.nominated_peer_id);
        println!("  Miner number: {}", block.miner_number);
    }

    // Demonstrate marking a block as orphaned
    println!("\nMarking block_hash_2 as orphaned...");

    if let Some(mut block) = MinerBlock::find_by_hash_multi(&datastore, "block_hash_2").await? {
        println!(
            "  Before: is_orphaned = {}, is_canonical = {}",
            block.is_orphaned, block.is_canonical
        );

        block.mark_as_orphaned(
            "Manual test orphaning".to_string(),
            Some("replacement_hash".to_string()),
        );

        println!(
            "  After:  is_orphaned = {}, is_canonical = {}",
            block.is_orphaned, block.is_canonical
        );

        // Save the updated block
        block.save_to_active(&datastore).await?;
        println!("  Saved updated block");
    }

    // Verify the change
    if let Some(block) = MinerBlock::find_by_hash_multi(&datastore, "block_hash_2").await? {
        println!("  Verified: is_orphaned = {}", block.is_orphaned);
    }

    // Statistics
    println!("\n📊 Final Statistics:");
    let all_canonical_epoch_0 =
        MinerBlock::find_canonical_by_epoch_multi(&datastore, 0, CURRENT_EPOCH).await?;
    let all_canonical_epoch_1 =
        MinerBlock::find_canonical_by_epoch_multi(&datastore, 1, CURRENT_EPOCH).await?;
    let all_orphaned = MinerBlock::find_all_orphaned_multi(&datastore).await?;

    println!(
        "  Canonical blocks in epoch 0: {}",
        all_canonical_epoch_0.len()
    );
    println!(
        "  Canonical blocks in epoch 1: {}",
        all_canonical_epoch_1.len()
    );
    println!("  Total orphaned blocks: {}", all_orphaned.len());

    println!("\n✓ Example completed successfully!");

    Ok(())
}
