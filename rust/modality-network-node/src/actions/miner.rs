use anyhow::Result;
use libp2p::gossipsub::IdentTopic;
use modality_network_datastore::Model;
use modality_network_datastore::models::MinerBlock;

use crate::node::Node;
use crate::gossip;

/// Run a mining node that continuously mines and gossips blocks
pub async fn run(node: &mut Node) -> Result<()> {
    // Subscribe to miner gossip
    gossip::add_miner_event_listeners(node).await?;

    // Start status server and networking
    node.start_status_server().await?;
    node.start_networking().await?;
    node.start_autoupgrade().await?;
    
    // Only wait for connections if we have bootstrappers configured
    if !node.bootstrappers.is_empty() {
        log::info!("Waiting for peer connections...");
        node.wait_for_connections().await?;
        
        // Sync from peers before starting to mine
        log::info!("Syncing blockchain state from peers...");
        if let Err(e) = sync_from_peers(node).await {
            log::warn!("Failed to sync from peers: {:?}. Starting with local chain.", e);
        }
    } else {
        log::info!("No bootstrappers configured - mining in solo mode");
    }

    log::info!("Starting miner...");
    
    // Get the current blockchain height from datastore
    let latest_block = {
        let datastore = node.datastore.lock().await;
        MinerBlock::find_all_canonical(&datastore).await?
            .into_iter()
            .max_by_key(|b| b.index)
    };

    let starting_index = match latest_block {
        Some(block) => {
            log::info!("Resuming mining from block index {}", block.index);
            block.index + 1
        }
        None => {
            log::info!("No existing blocks found, starting from genesis");
            0
        }
    };

    // Start sync listener task
    let sync_datastore = node.datastore.clone();
    let _sync_node = node.peerid;
    let sync_bootstrappers = node.bootstrappers.clone();
    let _sync_swarm = node.swarm.clone();
    let mut sync_trigger_rx = node.sync_trigger_tx.subscribe();
    
    tokio::spawn(async move {
        let mut last_sync_time = std::time::Instant::now();
        let sync_cooldown = std::time::Duration::from_secs(5);
        
        while let Ok(target_index) = sync_trigger_rx.recv().await {
            // Rate limit syncs
            if last_sync_time.elapsed() < sync_cooldown {
                log::debug!("Sync cooldown active, skipping");
                continue;
            }
            
            log::info!("🔄 Sync requested for blocks up to index {}", target_index);
            last_sync_time = std::time::Instant::now();
            
            // Find first available peer to sync from
            if let Some(_peer_addr) = sync_bootstrappers.first() {
                // Get our current height
                let local_height = {
                    let ds = sync_datastore.lock().await;
                    MinerBlock::find_all_canonical(&ds).await
                        .map(|blocks| blocks.len() as u64)
                        .unwrap_or(0)
                };
                
                if local_height < target_index {
                    log::info!("Syncing blocks from {} to {}", local_height, target_index);
                    
                    // Use simplified sync (placeholder for now)
                    match sync_blocks_simple(local_height, target_index).await {
                        Ok(count) if count > 0 => {
                            log::info!("✓ Successfully synced {} blocks", count);
                        }
                        Ok(_) => {
                            log::info!("Sync completed - blocks should be received via gossip");
                        }
                        Err(e) => {
                            log::warn!("Failed to sync blocks: {:?}", e);
                        }
                    }
                }
            } else {
                log::warn!("No peers available for sync");
            }
        }
    });

    // Start mining loop
    let datastore = node.datastore.clone();
    let swarm = node.swarm.clone();
    let peerid_str = node.peerid.to_string();
    let miner_nominees = node.miner_nominees.clone();
    
    tokio::spawn(async move {
        let mut current_index = starting_index;
        
        loop {
            log::info!("Mining block at index {}...", current_index);
            
            // Mine a block (this is a simplified version - in production you'd use the full mining chain)
            // For now, we'll just create a basic block structure
            match mine_and_gossip_block(
                current_index,
                &peerid_str,
                &miner_nominees,
                datastore.clone(),
                swarm.clone(),
            ).await {
                Ok(()) => {
                    log::info!("Successfully mined and gossipped block {}", current_index);
                    current_index += 1;
                }
                Err(e) => {
                    log::error!("Error mining block {}: {:?}", current_index, e);
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
            
            // Small delay between blocks
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    // Wait for shutdown signal
    node.wait_for_shutdown().await?;

    Ok(())
}

/// Sync blockchain state from connected peers before mining
async fn sync_from_peers(node: &Node) -> Result<()> {
    // Get our current chain height
    let local_height = {
        let datastore = node.datastore.lock().await;
        let blocks = MinerBlock::find_all_canonical(&datastore).await?;
        blocks.len() as u64
    };
    
    log::info!("Local chain height: {} blocks", local_height);
    
    // If we have no blocks and have bootstrappers, wait for gossip
    if local_height == 0 && !node.bootstrappers.is_empty() {
        log::info!("No local blocks. Waiting to receive blocks via gossip...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    Ok(())
}

/// Request missing blocks from a peer
#[allow(dead_code)]
async fn request_blocks_from_peer(
    _node: &Node,
    _peer_addr: &str,
    _from_index: u64,
    _to_index: u64,
) -> Result<usize> {
    // This function is not currently used but kept for future reference
    // when implementing active peer-to-peer sync
    Ok(0)
}

/// Simplified sync function for use in the sync listener task
async fn sync_blocks_simple(
    _from_index: u64,
    _to_index: u64,
) -> Result<usize> {
    // This is a simplified version that doesn't use the full node API
    // In production, you'd want to use the node's connection pool
    // For now, we log that sync was attempted
    log::debug!("Sync blocks placeholder called");
    Ok(0)
}

async fn mine_and_gossip_block(
    index: u64,
    peer_id: &str,
    miner_nominees: &Option<Vec<String>>,
    datastore: std::sync::Arc<tokio::sync::Mutex<modality_network_datastore::NetworkDatastore>>,
    swarm: std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
) -> Result<()> {
    use modality_network_mining::{Blockchain, ChainConfig};
    
    // Determine the nominee to use for this block
    let nominated_peer_id = match miner_nominees {
        Some(nominees) if !nominees.is_empty() => {
            // Select a nominee by rotating through the list based on block index
            let nominee_index = (index as usize) % nominees.len();
            nominees[nominee_index].clone()
        }
        _ => {
            // If no nominees are configured, use the miner's own peer ID
            peer_id.to_string()
        }
    };

    log::info!("Mining block {} with nominated peer: {}", index, nominated_peer_id);

    // Load blockchain from datastore with all historical blocks for proper difficulty calculation
    let mut chain = {
        let mut ds_guard = datastore.lock().await;
        use modality_network_mining::persistence::BlockchainPersistence;
        
        let loaded_blocks = ds_guard.load_canonical_blocks().await?;
        
        log::info!("Loaded {} blocks from datastore", loaded_blocks.len());
        
        if loaded_blocks.is_empty() {
            // No existing blocks, create genesis
            let chain = Blockchain::new(ChainConfig::default(), peer_id.to_string());
            
            // Save genesis block to datastore
            let genesis = &chain.blocks[0];
            let genesis_miner_block = MinerBlock::new_canonical(
                genesis.header.hash.clone(),
                genesis.header.index,
                0, // epoch 0
                genesis.header.timestamp.timestamp(),
                genesis.header.previous_hash.clone(),
                genesis.header.data_hash.clone(),
                genesis.header.nonce,
                genesis.header.difficulty,
                genesis.data.nominated_peer_id.clone(),
                genesis.data.miner_number,
            );
            genesis_miner_block.save(&mut ds_guard).await?;
            log::info!("Saved genesis block (index: 0) to datastore");
            
            drop(ds_guard); // Release the lock
            chain
        } else {
            // Reconstruct blockchain from loaded blocks
            // Start with a fresh chain and replace genesis, then add remaining blocks
            let mut chain = Blockchain::new(ChainConfig::default(), peer_id.to_string());
            
            // Replace the auto-generated genesis with the loaded one
            chain.blocks.clear();
            chain.blocks.push(loaded_blocks[0].clone());
            
            log::info!("Set genesis block (index: {}, hash: {})", 
                loaded_blocks[0].header.index,
                loaded_blocks[0].header.hash);
            
            // Add all subsequent blocks (add_block will handle block_index updates)
            for (i, block) in loaded_blocks.into_iter().skip(1).enumerate() {
                log::debug!("Adding block {} (index: {}, prev_hash: {}, hash: {})", 
                    i + 1,
                    block.header.index,
                    &block.header.previous_hash[..16],
                    &block.header.hash[..16]);
                chain.add_block(block)?;
            }
            
            log::info!("Reconstructed chain with {} blocks (height: {})", 
                chain.blocks.len(),
                chain.height());
            
            drop(ds_guard); // Release the lock
            chain
        }
    };
    
    // Check if we're trying to mine a block that already exists
    if index < chain.height() + 1 && index < chain.blocks.len() as u64 {
        // Block already exists in the chain, skip it
        log::warn!("Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
        return Ok(());
    }
    
    // Verify we're mining the correct next block
    let expected_next = chain.height() + 1;
    if index != expected_next {
        log::error!("Index mismatch: expected to mine block {}, but was asked to mine block {}", expected_next, index);
        return Err(anyhow::anyhow!("Index mismatch: chain expects block {} but trying to mine {}", expected_next, index));
    }
    
    log::info!("Chain ready for mining. Height: {}, Mining next index: {}", chain.height(), index);
    
    // Mine the next block (difficulty will be calculated based on loaded blockchain state)
    let miner_number = rand::random::<u64>();
    let mined_block = chain.mine_block(nominated_peer_id.clone(), miner_number)?;
    
    // Verify the mined block has the expected index
    if mined_block.header.index != index {
        log::error!("Mined block index mismatch: expected {}, got {}", index, mined_block.header.index);
        return Err(anyhow::anyhow!("Mined block index mismatch"));
    }

    // Convert to MinerBlock for datastore
    let miner_block = MinerBlock::new_canonical(
        mined_block.header.hash.clone(),
        index, // Use the passed index
        index / 40, // Assuming 40 blocks per epoch
        mined_block.header.timestamp.timestamp(),
        mined_block.header.previous_hash.clone(),
        mined_block.header.data_hash.clone(),
        mined_block.header.nonce,
        mined_block.header.difficulty,
        mined_block.data.nominated_peer_id.clone(),
        mined_block.data.miner_number,
    );

    // Save to datastore with duplicate checking and fork choice
    {
        let mut ds = datastore.lock().await;
        
        // Check if a block already exists at this index
        match MinerBlock::find_canonical_by_index(&ds, index).await? {
            Some(existing) => {
                // Block exists - apply fork choice (lower hash = harder to mine = wins)
                if miner_block.hash < existing.hash {
                    log::info!("Fork choice: Replacing existing block {} (hash: {}) with new block (hash: {})",
                        index, &existing.hash[..16], &miner_block.hash[..16]);
                    
                    // Mark old block as orphaned
                    let mut orphaned = existing.clone();
                    orphaned.mark_as_orphaned(
                        "Replaced by block with harder hash".to_string(),
                        Some(miner_block.hash.clone())
                    );
                    orphaned.save(&mut ds).await?;
                    
                    // Save new block as canonical
                    miner_block.save(&mut ds).await?;
                } else {
                    log::info!("Block {} already exists with equal or harder hash (existing: {}, new: {}), skipping save",
                        index, &existing.hash[..16], &miner_block.hash[..16]);
                }
            }
            None => {
                // No existing block at this index, save normally
                log::info!("Saving block {} (hash: {}) to datastore", miner_block.index, &miner_block.hash[..16]);
                miner_block.save(&mut ds).await?;
            }
        }
    }

    // Gossip the block
    let gossip_msg = gossip::miner::block::MinerBlockGossip::from_miner_block(&miner_block);
    let topic = IdentTopic::new(gossip::miner::block::TOPIC);
    let json = serde_json::to_string(&gossip_msg)?;
    
    {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock
            .behaviour_mut()
            .gossipsub
            .publish(topic, json.as_bytes())?;
    }

    log::info!("Mined block {} (epoch {}) with hash {} and difficulty {}", 
        miner_block.index,
        miner_block.epoch,
        &miner_block.hash[..16],
        miner_block.difficulty);
    
    // Log epoch changes prominently
    if miner_block.index > 0 && miner_block.index % 40 == 0 {
        log::info!("🎯 EPOCH {} STARTED - New difficulty: {}", miner_block.epoch, miner_block.difficulty);
    }

    Ok(())
}


