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

    // Start sync listener task that actually requests missing blocks
    let sync_datastore = node.datastore.clone();
    let sync_swarm = node.swarm.clone();
    let sync_bootstrappers = node.bootstrappers.clone();
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
            
            // Find the actual range of blocks we need
            let (first_index, _last_index) = {
                let ds = sync_datastore.lock().await;
                match MinerBlock::find_all_canonical(&ds).await {
                    Ok(blocks) => {
                        if blocks.is_empty() {
                            (0, target_index)
                        } else {
                            let min_index = blocks.iter().map(|b| b.index).min().unwrap_or(0);
                            let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
                            
                            // If we're missing genesis (block 0), start from 0
                            let start = if min_index > 0 { 0 } else { max_index + 1 };
                            (start, target_index)
                        }
                    }
                    Err(_) => (0, target_index)
                }
            };
            
            if first_index > target_index {
                log::debug!("Already have all blocks up to {}, no sync needed", target_index);
                continue;
            }
            
            log::info!("Requesting blocks from {} to {} from peers", first_index, target_index);
            
            // Try to get blocks from the first available peer
            if let Some(peer_addr) = sync_bootstrappers.first() {
                // Use reqres protocol to request block range
                match request_block_range_from_peer(
                    &sync_swarm,
                    peer_addr.to_string(),
                    first_index,
                    target_index,
                    &sync_datastore
                ).await {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Successfully synced {} blocks!", count);
                    }
                    Ok(_) => {
                        log::warn!("Sync request succeeded but no new blocks received");
                    }
                    Err(e) => {
                        log::warn!("Failed to sync blocks: {:?}", e);
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
            // Check if we have newer blocks from gossip before mining
            let latest_canonical = {
                let ds = datastore.lock().await;
                if let Ok(blocks) = MinerBlock::find_all_canonical(&ds).await {
                    blocks.into_iter().max_by_key(|b| b.index)
                } else {
                    None
                }
            };
            
            if let Some(latest) = latest_canonical {
                if latest.index >= current_index {
                    log::info!("📥 Detected newer blocks via gossip, updating mining index from {} to {}", 
                        current_index, latest.index + 1);
                    current_index = latest.index + 1;
                }
            }
            
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
                        let error_msg = e.to_string();
                        log::error!("Error mining block {}: {:?}", current_index, e);
                        
                        // Check if this is a chain divergence error
                        if error_msg.contains("Previous hash doesn't match") || 
                           error_msg.contains("Invalid block index") {
                            log::warn!("⚠️  Chain divergence detected at block {}! Validating chain...", current_index);
                            
                            // Acquire lock once for all operations
                            let mut ds = datastore.lock().await;
                            
                            if let Ok(all_blocks) = MinerBlock::find_all_canonical(&ds).await {
                                let max_index = all_blocks.iter().map(|b| b.index).max().unwrap_or(0);
                                
                                log::info!("Chain validation: have {} blocks, max index: {}, trying to mine: {}", 
                                    all_blocks.len(), max_index, current_index);
                                
                                // Validate the chain by checking if each block's previous_hash matches
                                // the hash of the actual previous block
                                let mut last_valid_index = 0;
                                let mut chain_is_valid = true;
                                
                                // First, check if we have block 0 (genesis)
                                if all_blocks.iter().find(|b| b.index == 0).is_none() {
                                    log::error!("❌ Missing genesis block (block 0)!");
                                    chain_is_valid = false;
                                    // last_valid_index stays at 0, meaning we have no valid chain
                                }
                                
                                for i in 1..=max_index {
                                    if let Some(block) = all_blocks.iter().find(|b| b.index == i) {
                                        if let Some(prev_block) = all_blocks.iter().find(|b| b.index == i - 1) {
                                            if block.previous_hash != prev_block.hash {
                                                log::error!(
                                                    "❌ Chain break detected at block {}: prev_hash {} doesn't match block {}'s hash {}",
                                                    i, &block.previous_hash[..16], i - 1, &prev_block.hash[..16]
                                                );
                                                chain_is_valid = false;
                                                break;
                                            }
                                            last_valid_index = i;
                                        } else {
                                            log::error!("❌ Missing block {} (gap in chain)", i - 1);
                                            chain_is_valid = false;
                                            break;
                                        }
                                    } else {
                                        log::error!("❌ Missing block {} (gap in chain)", i);
                                        chain_is_valid = false;
                                        break;
                                    }
                                }
                                
                                if !chain_is_valid || last_valid_index < max_index {
                                    log::error!(
                                        "❌ INVALID CHAIN DETECTED: Last valid block is {}, but have blocks up to {}",
                                        last_valid_index, max_index
                                    );
                                    
                                    // Special case: if we're missing genesis (block 0), orphan everything
                                    if last_valid_index == 0 && all_blocks.iter().find(|b| b.index == 0).is_none() {
                                        log::error!("❌ CRITICAL: Missing genesis block! Orphaning entire invalid chain...");
                                        let mut orphaned_count = 0;
                                        for block in all_blocks.iter() {
                                            log::info!("Orphaning block {} (hash: {}) - no valid genesis", 
                                                block.index, &block.hash[..16]);
                                            let mut orphaned = block.clone();
                                            orphaned.mark_as_orphaned(
                                                "Missing genesis block - orphaning entire chain".to_string(),
                                                None
                                            );
                                            if let Err(e) = orphaned.save(&mut *ds).await {
                                                log::error!("Failed to orphan block {}: {}", block.index, e);
                                            } else {
                                                orphaned_count += 1;
                                            }
                                        }
                                        log::info!("✓ Orphaned {} blocks. Will need to sync from genesis.", orphaned_count);
                                        current_index = 0;
                                    } else {
                                        log::info!("Cleaning up invalid chain by orphaning blocks after {}...", last_valid_index);
                                        
                                        // Orphan all blocks after the last valid one
                                        let mut orphaned_count = 0;
                                        for block in all_blocks.iter() {
                                            if block.index > last_valid_index {
                                                log::info!("Orphaning invalid block {} (hash: {})", 
                                                    block.index, &block.hash[..16]);
                                                let mut orphaned = block.clone();
                                                orphaned.mark_as_orphaned(
                                                    format!("Chain validation failed: removing blocks after index {}", last_valid_index),
                                                    None
                                                );
                                                if let Err(e) = orphaned.save(&mut *ds).await {
                                                    log::error!("Failed to orphan block {}: {}", block.index, e);
                                                } else {
                                                    orphaned_count += 1;
                                                }
                                            }
                                        }
                                        
                                        log::info!("✓ Cleaned up {} invalid blocks. Restarting from block {}", 
                                            orphaned_count, last_valid_index + 1);
                                        current_index = last_valid_index + 1;
                                    }
                                } else {
                                    log::warn!("Chain appears valid up to block {}, but mining still failed. This shouldn't happen!", 
                                        last_valid_index);
                                }
                            }
                            drop(ds);
                        }
                        
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

/// Request a range of blocks from a peer using the reqres protocol
async fn request_block_range_from_peer(
    swarm: &std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    from_index: u64,
    to_index: u64,
    datastore: &std::sync::Arc<tokio::sync::Mutex<modality_network_datastore::NetworkDatastore>>,
) -> Result<usize> {
    use libp2p::multiaddr::Multiaddr;
    use crate::reqres;
    
    let ma: Multiaddr = peer_addr.parse()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Invalid peer address - missing PeerID");
    };
    
    // Prepare the request
    let request = reqres::Request {
        path: "/data/miner_block/range".to_string(),
        data: Some(serde_json::json!({
            "from_index": from_index,
            "to_index": to_index
        })),
    };
    
    log::debug!("Sending block range request to {}", target_peer_id);
    
    // Send the request through swarm
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
    };
    
    log::debug!("Request sent with ID: {:?}", request_id);
    
    // Wait for response (with timeout)
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        wait_for_response(swarm, request_id)
    ).await??;
    
    if !response.ok {
        anyhow::bail!("Peer returned error: {:?}", response.errors);
    }
    
    // Save the blocks
    let saved_count = if let Some(ref data) = response.data {
        if let Some(blocks_array) = data.get("blocks").and_then(|b| b.as_array()) {
            let mut ds = datastore.lock().await;
            let mut count = 0;
            let mut skipped_no_parent = 0;
            
            for block_json in blocks_array {
                let block: MinerBlock = serde_json::from_value(block_json.clone())?;
                
                // Check if we already have this block
                if MinerBlock::find_by_hash(&*ds, &block.hash).await?.is_some() {
                    continue;
                }
                
                // Check if parent exists (except for genesis)
                if block.index > 0 {
                    match MinerBlock::find_by_hash(&*ds, &block.previous_hash).await? {
                        Some(_) => {
                            // Parent exists, save the block
                            block.save(&*ds).await?;
                            count += 1;
                            log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                        }
                        None => {
                            // Parent missing - this indicates chain divergence!
                            skipped_no_parent += 1;
                            log::warn!("Cannot save block {} - missing parent", block.index);
                        }
                    }
                } else {
                    // Genesis block
                    block.save(&*ds).await?;
                    count += 1;
                    log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                }
            }
            
            // Detect chain divergence
            if skipped_no_parent > 0 && count == 0 {
                log::error!(
                    "⚠️  CHAIN DIVERGENCE DETECTED: Received {} blocks but none could be saved due to missing parents. \
                    This node's chain has diverged from the peer's chain. Chain reorganization needed!",
                    skipped_no_parent
                );
                
                // Try to find common ancestor and perform reorg
                log::info!("Attempting chain reorganization...");
                if let Err(e) = attempt_chain_reorg(&mut ds, blocks_array, from_index).await {
                    log::error!("Chain reorg failed: {:?}", e);
                }
            }
            
            count
        } else {
            0
        }
    } else {
        0
    };
    
    Ok(saved_count)
}

/// Attempt to reorganize the chain when divergence is detected
async fn attempt_chain_reorg(
    ds: &mut modality_network_datastore::NetworkDatastore,
    peer_blocks: &[serde_json::Value],
    start_index: u64,
) -> Result<()> {
    use modality_network_datastore::Model;
    
    log::info!("Starting chain reorganization from index {}", start_index);
    
    // Find the common ancestor by going backwards
    let mut common_ancestor_index = None;
    
    for peer_block_json in peer_blocks.iter().rev() {
        let peer_block: MinerBlock = serde_json::from_value(peer_block_json.clone())?;
        
        // Check if we have a block at this index
        if let Some(local_block) = MinerBlock::find_canonical_by_index(ds, peer_block.index).await? {
            if local_block.hash == peer_block.hash {
                // Found common ancestor!
                common_ancestor_index = Some(peer_block.index);
                log::info!("Found common ancestor at block {}", peer_block.index);
                break;
            }
        }
    }
    
    match common_ancestor_index {
        Some(ancestor_index) => {
            log::info!("Reorganizing chain from block {} onwards", ancestor_index + 1);
            
            // Collect blocks that would be orphaned (local chain after ancestor)
            let all_local_blocks = MinerBlock::find_all_canonical(ds).await?;
            let orphan_candidates: Vec<_> = all_local_blocks
                .iter()
                .filter(|b| b.index > ancestor_index)
                .cloned()
                .collect();
            
            // Collect peer blocks after ancestor
            let mut peer_blocks_to_adopt: Vec<MinerBlock> = Vec::new();
            for peer_block_json in peer_blocks {
                let peer_block: MinerBlock = serde_json::from_value(peer_block_json.clone())?;
                if peer_block.index > ancestor_index {
                    peer_blocks_to_adopt.push(peer_block);
                }
            }
            
            // Compare cumulative difficulty of the two branches
            let local_branch_difficulty = MinerBlock::calculate_cumulative_difficulty(&orphan_candidates)?;
            let peer_branch_difficulty = MinerBlock::calculate_cumulative_difficulty(&peer_blocks_to_adopt)?;
            
            log::info!("Local branch (after block {}): {} blocks, cumulative difficulty: {}", 
                ancestor_index, orphan_candidates.len(), local_branch_difficulty);
            log::info!("Peer branch (after block {}): {} blocks, cumulative difficulty: {}", 
                ancestor_index, peer_blocks_to_adopt.len(), peer_branch_difficulty);
            
            if peer_branch_difficulty > local_branch_difficulty {
                log::info!("Peer branch has higher cumulative difficulty - adopting it");
                
                // Mark all blocks after the ancestor as orphaned
                let orphan_count = orphan_candidates.len();
                for local_block in orphan_candidates {
                    log::info!("Marking block {} as orphaned (reorg)", local_block.index);
                    let mut orphaned = local_block.clone();
                    orphaned.mark_as_orphaned(
                        format!("Chain reorganization: replaced by branch with higher cumulative difficulty ({} vs {})", 
                            peer_branch_difficulty, local_branch_difficulty),
                        None
                    );
                    orphaned.save(ds).await?;
                }
                
                // Save peer blocks starting after the ancestor
                let mut saved = 0;
                for peer_block in peer_blocks_to_adopt {
                    peer_block.save(ds).await?;
                    saved += 1;
                    log::debug!("Reorg: saved block {} at index {}", &peer_block.hash[..16], peer_block.index);
                }
                
                log::info!("✓ Chain reorganization complete: replaced {} blocks (difficulty {}) with {} new blocks (difficulty {})", 
                    orphan_count, local_branch_difficulty, saved, peer_branch_difficulty);
            } else {
                log::info!("Local branch has equal or higher cumulative difficulty - keeping it");
                anyhow::bail!("Local branch has equal or higher cumulative difficulty, no reorg needed");
            }
            
            Ok(())
        }
        None => {
            // No common ancestor found - chains have completely diverged
            // Apply cumulative difficulty rule: adopt the chain with more total work
            log::warn!("No common ancestor found - chains have completely diverged!");
            
            let local_blocks = MinerBlock::find_all_canonical(ds).await?;
            let local_chain_length = local_blocks.len();
            let peer_chain_length = peer_blocks.len();
            
            // Parse peer blocks
            let peer_blocks_parsed: Result<Vec<MinerBlock>, _> = peer_blocks
                .iter()
                .map(|json| serde_json::from_value(json.clone()))
                .collect();
            let peer_blocks_parsed = peer_blocks_parsed?;
            
            // Calculate cumulative difficulties
            let local_difficulty = MinerBlock::calculate_cumulative_difficulty(&local_blocks)?;
            let peer_difficulty = MinerBlock::calculate_cumulative_difficulty(&peer_blocks_parsed)?;
            
            log::info!("Local chain: {} blocks, cumulative difficulty: {}", local_chain_length, local_difficulty);
            log::info!("Peer chain: {} blocks, cumulative difficulty: {}", peer_chain_length, peer_difficulty);
            
            if peer_difficulty > local_difficulty {
                log::info!("Peer chain has higher cumulative difficulty - adopting it and orphaning entire local chain");
                
                // Orphan all local blocks
                for local_block in local_blocks {
                    log::info!("Marking block {} as orphaned (complete reorg)", local_block.index);
                    let mut orphaned = local_block.clone();
                    orphaned.mark_as_orphaned(
                        format!("Complete chain reorganization: no common ancestor, peer chain has higher cumulative difficulty ({} vs {})", 
                            peer_difficulty, local_difficulty),
                        None
                    );
                    orphaned.save(ds).await?;
                }
                
                // Save all peer blocks
                let mut saved = 0;
                for peer_block in peer_blocks_parsed {
                    peer_block.save(ds).await?;
                    saved += 1;
                    log::debug!("Reorg: saved block {} at index {}", &peer_block.hash[..16], peer_block.index);
                }
                
                log::info!("✓ Complete chain reorganization: replaced entire local chain ({} blocks, difficulty {}) with peer chain ({} blocks, difficulty {})", 
                    local_chain_length, local_difficulty, saved, peer_difficulty);
                
                Ok(())
            } else {
                log::info!("Local chain has equal or higher cumulative difficulty - keeping it");
                anyhow::bail!("Local chain has equal or higher cumulative difficulty, no reorg needed")
            }
        }
    }
}

/// Wait for a response from a request-response query
async fn wait_for_response(
    swarm: &std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
    request_id: libp2p::request_response::OutboundRequestId,
) -> Result<crate::reqres::Response> {
    use futures::StreamExt;
    
    loop {
        let event = {
            let mut swarm_lock = swarm.lock().await;
            swarm_lock.select_next_some().await
        };
        
        if let libp2p::swarm::SwarmEvent::Behaviour(
            crate::swarm::NodeBehaviourEvent::Reqres(
                libp2p::request_response::Event::Message {
                    message: libp2p::request_response::Message::Response { request_id: rid, response },
                    ..
                }
            )
        ) = event {
            if rid == request_id {
                return Ok(response);
            }
        }
    }
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
                // Block exists - apply fork choice (higher difficulty = more work = wins)
                let new_difficulty = mined_block.header.difficulty;
                let existing_difficulty = existing.get_difficulty_u128()?;
                
                if new_difficulty > existing_difficulty {
                    log::info!("Fork choice: Replacing existing block {} (difficulty: {}) with new block (difficulty: {})",
                        index, existing_difficulty, new_difficulty);
                    
                    // Mark old block as orphaned
                    let mut orphaned = existing.clone();
                    orphaned.mark_as_orphaned(
                        format!("Replaced by block with higher difficulty ({} vs {})", new_difficulty, existing_difficulty),
                        Some(miner_block.hash.clone())
                    );
                    orphaned.save(&mut ds).await?;
                    
                    // Save new block as canonical
                    miner_block.save(&mut ds).await?;
                } else {
                    log::info!("Block {} already exists with equal or higher difficulty (existing: {}, new: {}), skipping save",
                        index, existing_difficulty, new_difficulty);
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
    
    // Try to gossip the block, but don't fail if there are no peers (solo mining)
    {
        let mut swarm_lock = swarm.lock().await;
        match swarm_lock
            .behaviour_mut()
            .gossipsub
            .publish(topic, json.as_bytes()) {
            Ok(_) => {
                log::debug!("Gossipped block {} to peers", miner_block.index);
            }
            Err(e) => {
                log::debug!("Could not gossip block {} (no peers available): {}", miner_block.index, e);
            }
        }
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


