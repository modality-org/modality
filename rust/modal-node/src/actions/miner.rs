use anyhow::Result;
use libp2p::gossipsub::IdentTopic;
use modal_datastore::Model;
use modal_datastore::models::MinerBlock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::node::Node;
use crate::gossip;

/// Result of a mining operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningOutcome {
    /// Block was successfully mined and gossipped
    Mined,
    /// Block was skipped because it already exists
    Skipped,
}

/// Shared state for coordinating mining with sync operations
#[derive(Clone, Debug)]
struct MiningState {
    /// The index we're currently mining at
    current_mining_index: u64,
}

/// Run a mining node that continuously mines and gossips blocks
/// This function will run until a shutdown signal is received (Ctrl-C)
pub async fn run(node: &mut Node) -> Result<()> {
    // Ctrl-C handler is set up in node.wait_for_shutdown()
    // We just need a shutdown flag for mining operations
    let shutdown = Arc::new(AtomicBool::new(false));
    
    // Create a channel to signal mining view updates
    // This must be created FIRST so we can clone it for various tasks
    let (mining_update_tx, mut mining_update_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    
    // Store the mining update channel in node so gossip/networking handlers can use it
    node.mining_update_tx = Some(mining_update_tx.clone());
    // Start sync request handler task BEFORE gossip is active
    // This handles chain comparison requests triggered by orphan detection
    let sync_node_datastore = node.datastore.clone();
    let sync_node_swarm = node.swarm.clone();
    let sync_node_ignored_peers = node.ignored_peers.clone();
    let sync_node_reqres_txs = node.reqres_response_txs.clone();
    let (sync_request_tx, mut sync_request_rx) = tokio::sync::mpsc::unbounded_channel();
    // Set the node's channel so gossip handler can send to our new receiver
    node.sync_request_tx = Some(sync_request_tx);
    let mining_update_tx_for_sync = mining_update_tx.clone();
    
    // Shared flag to pause mining during sync
    let sync_in_progress = Arc::new(AtomicBool::new(false));
    let sync_in_progress_for_request = sync_in_progress.clone();
    
    // Track which peers are currently being synced to avoid duplicate requests
    let syncing_peers = Arc::new(Mutex::new(std::collections::HashSet::<libp2p::PeerId>::new()));
    let syncing_peers_clone = syncing_peers.clone();
    
    tokio::spawn(async move {
        while let Some((peer_id, peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut peers = syncing_peers_clone.lock().await;
                if peers.contains(&peer_id) {
                    log::debug!("Sync already in progress for peer {}, skipping duplicate request", peer_id);
                    continue;
                }
                peers.insert(peer_id);
            }
            
            // Signal mining to pause during sync
            sync_in_progress_for_request.store(true, Ordering::Relaxed);
            
            log::info!("🔄 Processing sync request for peer {}", peer_id);
            
            // Create a temporary mutable node-like structure for request_chain_info
            // Since we can't easily clone Node, we'll call the function with what it needs
            let result = request_chain_info_impl(
                peer_id,
                peer_addr,
                sync_node_swarm.clone(),
                sync_node_datastore.clone(),
                sync_node_ignored_peers.clone(),
                sync_node_reqres_txs.clone(),
            ).await;
            
            match result {
                Ok(()) => {
                    // After successful chain sync, notify mining loop to update view
                    let new_tip = {
                        let ds = sync_node_datastore.lock().await;
                        match MinerBlock::find_all_canonical(&ds).await {
                            Ok(blocks) if !blocks.is_empty() => {
                                blocks.iter().map(|b| b.index).max()
                            }
                            _ => None
                        }
                    };
                    if let Some(tip) = new_tip {
                        log::info!("📡 Chain sync completed, notifying mining loop (new tip: {})", tip);
                        let _ = mining_update_tx_for_sync.send(tip);
                    }
                }
                Err(e) => {
                    log::warn!("Chain sync failed for peer {}: {}", peer_id, e);
                }
            }
            
            // Remove peer from syncing set and resume mining
            {
                let mut peers = syncing_peers_clone.lock().await;
                peers.remove(&peer_id);
            }
            sync_in_progress_for_request.store(false, Ordering::Relaxed);
        }
    });

    // Subscribe to miner gossip AFTER setting up sync channel
    gossip::add_miner_event_listeners(node).await?;

    // Start status server and networking
    node.start_status_server().await?;
    node.start_status_html_writer().await?;
    node.start_networking().await?;
    node.start_autoupgrade().await?;
    
    // Get the current blockchain height from datastore (before we start syncing)
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

    // Start sync listener task BEFORE we wait for connections
    // This ensures it's ready to handle sync triggers from gossip
    // This task also handles chain cleanup/reorg in the background
    let sync_datastore = node.datastore.clone();
    let sync_swarm = node.swarm.clone();
    let sync_bootstrappers = node.bootstrappers.clone();
    let sync_reqres_txs = node.reqres_response_txs.clone();
    let mut sync_trigger_rx = node.sync_trigger_tx.subscribe();
    let mining_update_tx_clone = mining_update_tx.clone();
    
    // Use the same sync flag for the background sync listener
    let sync_in_progress_clone = sync_in_progress.clone();
    
    tokio::spawn(async move {
        let mut last_sync_time = std::time::Instant::now();
        // Reduced from 5s to 500ms for faster convergence during active mining
        let sync_cooldown = std::time::Duration::from_millis(500);
        
        while let Ok(target_index) = sync_trigger_rx.recv().await {
            // Rate limit syncs (but much faster than before)
            if last_sync_time.elapsed() < sync_cooldown {
                log::debug!("Sync cooldown active ({}ms remaining)", 
                    sync_cooldown.as_millis().saturating_sub(last_sync_time.elapsed().as_millis()));
                continue;
            }
            
            // Signal mining to pause
            sync_in_progress_clone.store(true, Ordering::Relaxed);
            
            log::info!("🔄 Sync requested for blocks up to index {}", target_index);
            last_sync_time = std::time::Instant::now();
            
            // Step 1: Validate and clean up local chain if needed
            // This runs in background and doesn't block mining
            {
                let mut ds = sync_datastore.lock().await;
                
                if let Ok(all_blocks) = MinerBlock::find_all_canonical(&ds).await {
                    if !all_blocks.is_empty() {
                        let max_index = all_blocks.iter().map(|b| b.index).max().unwrap_or(0);
                        
                        // Quick chain validation
                        let mut last_valid_index = 0;
                        let mut chain_is_valid = true;
                        
                        // Check if we have block 0 (genesis)
                        if all_blocks.iter().find(|b| b.index == 0).is_none() {
                            log::warn!("⚠️  Missing genesis block during sync validation");
                            chain_is_valid = false;
                        } else {
                            // Validate chain continuity
                            for i in 1..=max_index {
                                if let Some(block) = all_blocks.iter().find(|b| b.index == i) {
                                    if let Some(prev_block) = all_blocks.iter().find(|b| b.index == i - 1) {
                                        if block.previous_hash != prev_block.hash {
                                            log::warn!(
                                                "⚠️  Chain break at block {}: prev_hash {} doesn't match block {}'s hash {}",
                                                i, &block.previous_hash[..16], i - 1, &prev_block.hash[..16]
                                            );
                                            chain_is_valid = false;
                                            break;
                                        }
                                        last_valid_index = i;
                                    } else {
                                        log::warn!("⚠️  Missing block {} (gap in chain)", i - 1);
                                        chain_is_valid = false;
                                        break;
                                    }
                                } else {
                                    log::warn!("⚠️  Missing block {} (gap in chain)", i);
                                    chain_is_valid = false;
                                    break;
                                }
                            }
                        }
                        
                        // Clean up invalid blocks if needed
                        if !chain_is_valid {
                            log::info!("🔧 Cleaning up invalid chain in background (last valid: {})", last_valid_index);
                            
                            let mut orphaned_count = 0;
                            for block in all_blocks.iter() {
                                if block.index > last_valid_index {
                                    let mut orphaned = block.clone();
                                    orphaned.mark_as_orphaned(
                                        format!("Background chain cleanup: removing blocks after index {}", last_valid_index),
                                        None
                                    );
                                    if let Err(e) = orphaned.save(&mut *ds).await {
                                        log::error!("Failed to orphan block {}: {}", block.index, e);
                                    } else {
                                        orphaned_count += 1;
                                    }
                                }
                            }
                            
                            if orphaned_count > 0 {
                                log::info!("✓ Cleaned up {} invalid blocks in background", orphaned_count);
                                // Notify mining loop to update its view
                                let _ = mining_update_tx_clone.send(last_valid_index);
                            }
                        }
                    }
                }
            }
            
            // Step 2: Sync missing blocks from peers
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
                            let start = if min_index > 0 {
                                log::info!("Missing genesis - requesting full chain from 0");
                                0
                            } else if target_index > max_index + 1 {
                                // ANY gap suggests potential chain divergence - request from 0 for safety
                                log::warn!(
                                    "⚠️  Gap detected: we have blocks up to {}, but received orphan at index {}",
                                    max_index, target_index
                                );
                                log::info!("This suggests potential chain divergence - requesting full peer chain from 0 for comparison");
                                0
                            } else {
                                max_index + 1
                            };
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
                    &sync_datastore,
                    &sync_reqres_txs
                ).await {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Successfully synced {} blocks!", count);
                        // Notify mining loop that chain tip may have changed
                        let new_tip = {
                            let ds = sync_datastore.lock().await;
                            match MinerBlock::find_all_canonical(&ds).await {
                                Ok(blocks) if !blocks.is_empty() => {
                                    blocks.iter().map(|b| b.index).max()
                                }
                                _ => None
                            }
                        };
                        if let Some(tip) = new_tip {
                            let _ = mining_update_tx_clone.send(tip);
                        }
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
            
            // Resume mining after sync completes
            sync_in_progress_clone.store(false, Ordering::Relaxed);
        }
    });

    // Start mining loop - this runs CONTINUOUSLY without blocking
    let datastore = node.datastore.clone();
    let swarm = node.swarm.clone();
    let peerid_str = node.peerid.to_string();
    let miner_nominees = node.miner_nominees.clone();
    let fork_config = node.fork_config.clone();
    let mining_metrics = node.mining_metrics.clone();
    let initial_difficulty = node.initial_difficulty;
    let miner_hash_func = node.miner_hash_func.clone();
    let miner_hash_params = node.miner_hash_params.clone();
    let mining_delay_ms = node.mining_delay_ms;
    let epoch_transition_tx = if node.hybrid_consensus {
        Some(node.epoch_transition_tx.clone())
    } else {
        None
    };
    
    // Create shared mining state
    let mining_state = Arc::new(Mutex::new(MiningState {
        current_mining_index: starting_index,
    }));
    let mining_state_clone = mining_state.clone();
    
    // Clone shutdown flag for mining loop
    let shutdown_for_mining = shutdown.clone();
    
    tokio::spawn(async move {
        let mut current_index = starting_index;
        
        loop {
            // Check for shutdown signal
            if shutdown_for_mining.load(Ordering::Relaxed) {
                log::info!("🛑 Mining loop shutting down gracefully...");
                break;
            }
            
            // Check if sync is in progress and pause mining if so
            if sync_in_progress.load(Ordering::Relaxed) {
                log::debug!("⏸️  Mining paused - sync in progress");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
            
            // Non-blocking check for view updates from sync/gossip
            while let Ok(new_tip_index) = mining_update_rx.try_recv() {
                let next_index = new_tip_index + 1;
                if next_index > current_index {
                    log::info!("⛏️  Mining view updated: switching from block {} to block {}", 
                        current_index, next_index);
                    current_index = next_index;
                } else if next_index < current_index {
                    log::warn!("⛏️  Mining view updated: reorg detected, switching from block {} to block {}", 
                        current_index, next_index);
                    current_index = next_index;
                }
            }
            
            // Always get the latest canonical view before mining
            // This is a quick check that doesn't block
            let latest_tip_index = {
                let ds = datastore.lock().await;
                match MinerBlock::find_all_canonical(&ds).await {
                    Ok(blocks) if !blocks.is_empty() => {
                        blocks.into_iter().max_by_key(|b| b.index).map(|b| b.index)
                    }
                    _ => None
                }
            };
            
            // Update mining index if we see a newer tip
            if let Some(tip_index) = latest_tip_index {
                let next_index = tip_index + 1;
                if next_index > current_index {
                    log::info!("⛏️  Detected newer chain tip via datastore check: updating from {} to {}", 
                        current_index, next_index);
                    current_index = next_index;
                }
            }
            
            log::info!("⛏️  Mining block at index {}...", current_index);
            
            // Mine a block - if this fails, we simply retry on the next iteration
            // We don't block or do complex error handling here
            match mine_and_gossip_block(
                current_index,
                &peerid_str,
                &miner_nominees,
                datastore.clone(),
                swarm.clone(),
                fork_config.clone(),
                mining_metrics.clone(),
                initial_difficulty,
                miner_hash_func.clone(),
                miner_hash_params.clone(),
                mining_delay_ms,
                epoch_transition_tx.clone(),
            ).await {
                Ok(MiningOutcome::Mined) => {
                    log::info!("✅ Successfully mined and gossipped block {}", current_index);
                    // Move to next block
                    current_index += 1;
                    
                    // Update shared state
                    let mut state = mining_state_clone.lock().await;
                    state.current_mining_index = current_index;
                }
                Ok(MiningOutcome::Skipped) => {
                    log::info!("⏭️  Block {} already exists (received via gossip), moving to next block", current_index);
                    
                    // When we skip a block, we need to verify what the actual chain tip is
                    // to avoid getting stuck in a loop where we keep trying to mine on an orphaned branch
                    let actual_next_index = {
                        let ds = datastore.lock().await;
                        match MinerBlock::find_all_canonical(&ds).await {
                            Ok(blocks) if !blocks.is_empty() => {
                                let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
                                max_index + 1
                            }
                            _ => 0 // Start from genesis if no valid blocks
                        }
                    };
                    
                    // Use the queried index instead of just incrementing
                    current_index = actual_next_index;
                    log::info!("📍 Verified next mining index: {}", current_index);
                    
                    // Update shared state
                    let mut state = mining_state_clone.lock().await;
                    state.current_mining_index = current_index;
                }
                Err(e) => {
                    // Check if shutdown was signaled - if so, exit immediately
                    if shutdown_for_mining.load(Ordering::Relaxed) {
                        log::info!("🛑 Mining loop received shutdown signal during error handling, exiting");
                        break;
                    }
                    
                    // Log error but don't block - we'll retry on next iteration
                    // The sync/gossip handlers running in parallel will fix any chain issues
                    log::warn!("⚠️  Failed to mine block {} ({}), will retry with updated view", 
                        current_index, e);
                    
                    // Brief pause before retrying - give sync handlers time to update view
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    // Check if we need to adjust our view
                    // This happens when our local chain has issues
                    let corrected_index = {
                        let ds = datastore.lock().await;
                        match MinerBlock::find_all_canonical(&ds).await {
                            Ok(blocks) if !blocks.is_empty() => {
                                let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
                                Some(max_index + 1)
                            }
                            _ => Some(0) // Start from genesis if no valid blocks
                        }
                    };
                    
                    if let Some(corrected) = corrected_index {
                        if corrected != current_index {
                            log::info!("⛏️  Correcting mining index from {} to {} after error", 
                                current_index, corrected);
                            current_index = corrected;
                        }
                    }
                }
            }
            
            // Small delay between mining attempts
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Only wait for connections if we have bootstrappers configured
    // This happens AFTER the sync listener is started so it can handle sync triggers
    if !node.bootstrappers.is_empty() {
        log::info!("Waiting for peer connections...");
        node.wait_for_connections().await?;
        
        // Announce our chain tip to peers so they can sync if needed
        log::info!("Announcing our chain to connected peers...");
        if let Err(e) = announce_chain_tip(node).await {
            log::warn!("Failed to announce chain tip: {:?}", e);
        }
        
        // Sync from peers before starting to mine
        log::info!("Syncing blockchain state from peers...");
        if let Err(e) = sync_from_peers(node).await {
            log::warn!("Failed to sync from peers: {:?}. Starting with local chain.", e);
        }
    } else {
        log::info!("No bootstrappers configured - mining in solo mode");
    }

    log::info!("Starting miner...");

    // Store shutdown flag in node so wait_for_shutdown can check it
    node.mining_shutdown = Some(shutdown.clone());

    // Wait for shutdown signal
    node.wait_for_shutdown().await?;
    
    log::info!("🛑 Miner shutdown complete");

    Ok(())
}

/// Request chain info from a peer and perform sync if their chain has higher cumulative difficulty
/// This is called when we detect an orphan block from a peer
/// 
/// Uses the efficient find_ancestor reqres route to find common ancestor via binary search
pub async fn request_chain_info_impl(
    peer_id: libp2p::PeerId,
    peer_addr: String,
    swarm: std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
    datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    ignored_peers: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<libp2p::PeerId, crate::node::IgnoredPeerInfo>>>,
    reqres_response_txs: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
) -> Result<()> {
    // Check if peer is ignored
    {
        let ignored = ignored_peers.lock().await;
        if let Some(info) = ignored.get(&peer_id) {
            if std::time::Instant::now() < info.ignore_until {
                log::debug!("Peer {} is ignored, skipping chain info request", peer_id);
                return Ok(());
            }
        }
    }
    
    log::info!("🔄 Syncing with peer {} using efficient find_ancestor", peer_id);
    
    // Step 1: Find common ancestor using efficient binary search
    let (common_ancestor, peer_chain_length, peer_cumulative_difficulty) = match find_common_ancestor_efficient(&swarm, peer_addr.clone(), &datastore, &reqres_response_txs).await {
        Ok(info) => info,
        Err(e) => {
            log::warn!("Failed to find common ancestor with peer {}: {}", peer_id, e);
            return Ok(());
        }
    };
    
    // Step 2: Determine which blocks to request
    let from_index = match common_ancestor {
        Some(ancestor_index) => {
            log::info!("✓ Found common ancestor at index {}", ancestor_index);
            ancestor_index + 1 // Request blocks after the common ancestor
        }
        None => {
            log::warn!("⚠️  No common ancestor found - chains completely diverged");
            0 // Request full chain from genesis
        }
    };
    
    // Get our local chain info for comparison
    let (local_cumulative_difficulty, local_chain_length) = {
        let ds = datastore.lock().await;
        let blocks = MinerBlock::find_all_canonical(&ds).await?;
        let local_difficulty = MinerBlock::calculate_cumulative_difficulty(&blocks)?;
        (local_difficulty, blocks.len() as u64)
    };
    
    // Step 3: Request blocks from peer starting from the divergence point
    // Request in chunks of 100 blocks to avoid response size issues
    log::info!("📥 Requesting blocks from index {} onwards from peer", from_index);
    
    use libp2p::multiaddr::Multiaddr;
    let ma: Multiaddr = match peer_addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("Failed to parse peer address '{}': {}", peer_addr, e);
            return Ok(());
        }
    };
    
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        log::error!("Invalid peer address - missing PeerID: {}", peer_addr);
        return Ok(());
    };
    
    let mut all_blocks: Vec<MinerBlock> = Vec::new();
    let mut current_from = from_index;
    let chunk_size = 50; // Reduced from 100 for faster responses
    
    // Request blocks in chunks
    loop {
        log::debug!("Requesting blocks {}..{} from peer", current_from, current_from + chunk_size);
        
        let request = crate::reqres::Request {
            path: "/data/miner_block/range".to_string(),
            data: Some(serde_json::json!({
                "from_index": current_from,
                "to_index": current_from + chunk_size
            })),
        };
        
        let request_id = {
            let mut swarm_lock = swarm.lock().await;
            swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
        };
        
        log::debug!("Block range request sent with ID: {:?}", request_id);
        
        // Wait for response with timeout (60s to account for networking task contention)
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            wait_for_reqres_response(&reqres_response_txs, request_id)
        ).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                log::warn!("Failed to get block range from peer {}: {}", peer_id, e);
                break;
            }
            Err(_) => {
                log::warn!("Block range request to peer {} timed out", peer_id);
                break;
            }
        };
        
        if !response.ok {
            log::warn!("Peer returned error for block range: {:?}", response.errors);
            break;
        }
        
        let Some(ref data) = response.data else {
            log::warn!("Peer returned no data for block range");
            break;
        };
        
        // Parse blocks from response
        let Some(blocks_json) = data.get("blocks").and_then(|b| b.as_array()) else {
            log::warn!("No blocks array in response");
            break;
        };
        
        if blocks_json.is_empty() {
            log::info!("No more blocks available from peer");
            break;
        }
        
        log::info!("Received {} blocks from peer (indices {}..{})", 
            blocks_json.len(), current_from, current_from + blocks_json.len() as u64);
        
        // Parse and add blocks
        for block_json in blocks_json {
            match serde_json::from_value(block_json.clone()) {
                Ok(block) => all_blocks.push(block),
                Err(e) => {
                    log::warn!("Failed to parse block: {}", e);
                }
            }
        }
        
        // Check if there are more blocks to fetch
        let has_more = data.get("has_more").and_then(|v| v.as_bool()).unwrap_or(false);
        if !has_more {
            log::info!("Received all available blocks from peer");
            break;
        }
        
        current_from += blocks_json.len() as u64;
    }
    
    if all_blocks.is_empty() {
        log::info!("No blocks received from peer");
        return Ok(());
    }
    
    log::info!("Total blocks received: {}", all_blocks.len());
    
    // Step 4: Compare using peer's FULL chain cumulative difficulty (not just the fetched blocks)
    log::info!(
        "Chain comparison: Local (length: {}, difficulty: {}) vs Peer FULL chain (length: {}, difficulty: {})",
        local_chain_length, local_cumulative_difficulty,
        peer_chain_length, peer_cumulative_difficulty
    );
    
    // Compare cumulative difficulty, using chain length as tiebreaker
    let should_adopt_peer = peer_cumulative_difficulty > local_cumulative_difficulty ||
        (peer_cumulative_difficulty == local_cumulative_difficulty && 
         peer_chain_length > local_chain_length);
    
    if !should_adopt_peer {
        log::info!(
            "Keeping local chain (difficulty: {}, length: {}) over peer (difficulty: {}, length: {})",
            local_cumulative_difficulty, local_chain_length,
            peer_cumulative_difficulty, peer_chain_length
        );
        // Clean up pending blocks
        let ds = datastore.lock().await;
        let _ = MinerBlock::delete_all_pending(&ds).await;
        return Ok(());
    }
    
    log::info!("✅ Peer chain has higher cumulative difficulty - adopting it");
    
    // Step 5: Save blocks as pending first
    {
        let mut ds = datastore.lock().await;
        for block in &all_blocks {
            block.save_as_pending(&mut ds).await?;
        }
        log::debug!("Saved {} blocks as pending for verification", all_blocks.len());
    }
    
    // Step 6: Verify blocks form a valid chain
    // Ensure blocks are consecutive and properly linked
    all_blocks.sort_by_key(|b| b.index);
    for i in 1..all_blocks.len() {
        if all_blocks[i].index != all_blocks[i-1].index + 1 {
            log::error!("⚠️  Blocks not consecutive: gap between {} and {}", 
                all_blocks[i-1].index, all_blocks[i].index);
            let ds = datastore.lock().await;
            let _ = MinerBlock::delete_all_pending(&ds).await;
            return Ok(());
        }
        if all_blocks[i].previous_hash != all_blocks[i-1].hash {
            log::error!("⚠️  Invalid chain: block {} prev_hash doesn't match block {} hash",
                all_blocks[i].index, all_blocks[i-1].index);
            let ds = datastore.lock().await;
            let _ = MinerBlock::delete_all_pending(&ds).await;
            return Ok(());
        }
    }
    
    log::info!("✓ Peer chain validation passed");
    
    // Step 7: Orphan competing local blocks and canonize peer blocks
    {
        let mut ds = datastore.lock().await;
        
        // Find local blocks that compete with peer blocks
        let local_blocks = MinerBlock::find_all_canonical(&ds).await?;
        
        for block in &all_blocks {
            if let Some(local) = local_blocks.iter().find(|b| b.index == block.index && b.hash != block.hash) {
                log::info!("Orphaning local block {} at index {}", &local.hash[..16], local.index);
                let mut orphaned = local.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by peer chain with higher cumulative difficulty ({} vs {})", 
                        peer_cumulative_difficulty, local_cumulative_difficulty),
                    Some(block.hash.clone())
                );
                orphaned.save(&mut ds).await?;
            }
        }
        
        // Canonize all pending blocks
        for block in &mut all_blocks {
            block.canonize(&mut ds).await?;
        }
    }
    
    log::info!("🎉 Successfully adopted peer's chain with {} blocks!", all_blocks.len());
    
    Ok(())
}

/// Helper to wait for a reqres response using channels (no swarm lock contention)
async fn wait_for_reqres_response(
    node_reqres_txs: &std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
    request_id: libp2p::request_response::OutboundRequestId,
) -> Result<crate::reqres::Response> {
    // Create a oneshot channel for this request
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    // Register the channel
    {
        let mut txs = node_reqres_txs.lock().await;
        txs.insert(request_id, tx);
    }
    
    // Wait for the response from the networking task
    rx.await.map_err(|_| anyhow::anyhow!("Response channel closed"))
}

/// Announce our chain tip to connected peers
async fn announce_chain_tip(node: &Node) -> Result<()> {
    use crate::gossip;
    
    // Get our highest block
    let tip_block = {
        let datastore = node.datastore.lock().await;
        MinerBlock::find_all_canonical(&datastore).await?
            .into_iter()
            .max_by_key(|b| b.index)
    };
    
    if let Some(block) = tip_block {
        log::info!("Announcing chain tip: block {} (index: {})", &block.hash[..16], block.index);
        
        // Gossip the tip block to trigger sync on peers if needed
        let gossip_msg = gossip::miner::block::MinerBlockGossip::from_miner_block(&block);
        let topic = IdentTopic::new(gossip::miner::block::TOPIC);
        let json = serde_json::to_string(&gossip_msg)?;
        
        let mut swarm_lock = node.swarm.lock().await;
        match swarm_lock
            .behaviour_mut()
            .gossipsub
            .publish(topic, json.as_bytes()) {
            Ok(_) => {
                log::info!("✓ Announced our chain tip (block {}) to peers", block.index);
            }
            Err(e) => {
                log::debug!("Could not gossip chain tip: {}", e);
            }
        }
    } else {
        log::info!("No blocks to announce (empty chain)");
    }
    
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
    datastore: &std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    reqres_response_txs: &std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
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
    
    // Wait for response (with timeout - 60s to handle networking task contention)
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        wait_for_reqres_response(&reqres_response_txs, request_id)
    ).await??;
    
    if !response.ok {
        anyhow::bail!("Peer returned error: {:?}", response.errors);
    }
    
    // Save the blocks
    let saved_count = if let Some(ref data) = response.data {
        if let Some(blocks_array) = data.get("blocks").and_then(|b: &serde_json::Value| b.as_array()) {
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
                            // Parent exists, now check for fork choice
                            match MinerBlock::find_canonical_by_index(&*ds, block.index).await? {
                                Some(existing) => {
                                    // Apply fork choice: higher difficulty wins
                                    let new_difficulty = block.get_difficulty_u128()?;
                                    let existing_difficulty = existing.get_difficulty_u128()?;
                                    
                                    if new_difficulty > existing_difficulty {
                                        log::info!("Fork choice during sync: Replacing existing block {} (difficulty: {}) with synced block (difficulty: {})",
                                            block.index, existing_difficulty, new_difficulty);
                                        
                                        // Mark old block as orphaned
                                        let mut orphaned = existing.clone();
                                        orphaned.mark_as_orphaned(
                                            format!("Replaced by synced block with higher difficulty ({} vs {})", new_difficulty, existing_difficulty),
                                            Some(block.hash.clone())
                                        );
                                        orphaned.save(&mut *ds).await?;
                                        
                                        // Save new block as canonical
                                        block.save(&mut *ds).await?;
                                        count += 1;
                                        log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                                    } else {
                                        log::debug!("Existing block {} has equal or higher difficulty, skipping synced block", block.index);
                                    }
                                }
                                None => {
                                    // No existing block at this index, save it
                                    block.save(&mut *ds).await?;
                                    count += 1;
                                    log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                                }
                            }
                        }
                        None => {
                            // Parent missing - this indicates chain divergence!
                            skipped_no_parent += 1;
                            log::warn!("Cannot save block {} - missing parent", block.index);
                        }
                    }
                } else {
                    // Genesis block (index 0) - apply fork choice
                    match MinerBlock::find_canonical_by_index(&*ds, 0).await? {
                        Some(existing) => {
                            // Apply fork choice: higher difficulty wins (or lower hash as tiebreaker)
                            let new_difficulty = block.get_difficulty_u128()?;
                            let existing_difficulty = existing.get_difficulty_u128()?;
                            
                            if new_difficulty > existing_difficulty || 
                               (new_difficulty == existing_difficulty && block.hash < existing.hash) {
                                log::info!("Fork choice during sync: Replacing existing genesis block (hash: {}) with synced genesis (hash: {})",
                                    &existing.hash[..16], &block.hash[..16]);
                                
                                // Mark old genesis as orphaned
                                let mut orphaned = existing.clone();
                                orphaned.mark_as_orphaned(
                                    "Replaced by synced genesis block".to_string(),
                                    Some(block.hash.clone())
                                );
                                orphaned.save(&mut *ds).await?;
                                
                                // Save new genesis as canonical
                                block.save(&mut *ds).await?;
                                count += 1;
                                log::debug!("Saved synced genesis block {}", &block.hash[..16]);
                            } else {
                                log::debug!("Existing genesis has equal or higher difficulty/lower hash, skipping synced genesis");
                            }
                        }
                        None => {
                            // No existing genesis, save it
                            block.save(&mut *ds).await?;
                            count += 1;
                            log::debug!("Saved synced block {} (index: {})", &block.hash[..16], block.index);
                        }
                    }
                }
            }
            
            // Detect chain divergence and trigger reorg if needed
            if from_index == 0 && blocks_array.len() > 0 {
                // We requested from 0 (full chain comparison)
                // Check if the peer's chain is different from ours
                let local_blocks = MinerBlock::find_all_canonical(&ds).await?;
                let local_chain_length = local_blocks.len();
                let peer_chain_length = blocks_array.len();
                
                if peer_chain_length > local_chain_length {
                    log::warn!(
                        "⚠️  FULL CHAIN COMPARISON: Peer has longer chain ({} blocks) than us ({} blocks). Attempting reorg...",
                        peer_chain_length, local_chain_length
                    );
                    if let Err(e) = attempt_chain_reorg(&mut ds, blocks_array, from_index).await {
                        log::error!("Chain reorg failed: {:?}", e);
                    } else {
                        // Recount saved blocks after reorg
                        return Ok(MinerBlock::find_all_canonical(&ds).await?.len());
                    }
                } else if skipped_no_parent > 0 && count == 0 {
                    // Only do reorg if no blocks were saved due to missing parents
                    log::error!(
                        "⚠️  CHAIN DIVERGENCE DETECTED: Received {} blocks but none could be saved due to missing parents.",
                        skipped_no_parent
                    );
                    log::info!("Attempting chain reorganization...");
                    if let Err(e) = attempt_chain_reorg(&mut ds, blocks_array, from_index).await {
                        log::error!("Chain reorg failed: {:?}", e);
                    }
                }
            } else if skipped_no_parent > 0 && count == 0 {
                // Not a full chain comparison, but detected missing parents
                log::error!(
                    "⚠️  CHAIN DIVERGENCE DETECTED: Received {} blocks but none could be saved due to missing parents.",
                    skipped_no_parent
                );
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
    ds: &mut modal_datastore::NetworkDatastore,
    peer_blocks: &[serde_json::Value],
    start_index: u64,
) -> Result<()> {
    use modal_datastore::Model;
    
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
            } else if peer_branch_difficulty == local_branch_difficulty {
                // Cumulative difficulty is equal, use chain length as tiebreaker
                if peer_blocks_to_adopt.len() > orphan_candidates.len() {
                    log::info!("Equal difficulty but peer branch is longer - adopting it");
                    
                    // Mark all blocks after the ancestor as orphaned
                    let orphan_count = orphan_candidates.len();
                    for local_block in orphan_candidates {
                        log::info!("Marking block {} as orphaned (reorg - longer chain)", local_block.index);
                        let mut orphaned = local_block.clone();
                        orphaned.mark_as_orphaned(
                            "Chain reorganization: replaced by longer chain with equal difficulty".to_string(),
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
                    
                    log::info!("✓ Chain reorganization complete: replaced {} blocks with {} new blocks (longer chain)", 
                        orphan_count, saved);
                } else if peer_blocks_to_adopt.len() == orphan_candidates.len() {
                    // Same difficulty AND same length - use hash of first diverging block as tiebreaker
                    let peer_first_hash = peer_blocks_to_adopt.first().map(|b| &b.hash);
                    let local_first_hash = orphan_candidates.first().map(|b| &b.hash);
                    
                    if let (Some(ph), Some(lh)) = (peer_first_hash, local_first_hash) {
                        if ph < lh {
                            log::info!("Equal difficulty and length but peer branch has lower hash - adopting it");
                            
                            // Mark all blocks after the ancestor as orphaned
                            let orphan_count = orphan_candidates.len();
                            for local_block in orphan_candidates {
                                log::info!("Marking block {} as orphaned (reorg - hash tiebreaker)", local_block.index);
                                let mut orphaned = local_block.clone();
                                orphaned.mark_as_orphaned(
                                    "Chain reorganization: replaced by chain with lower hash (tiebreaker)".to_string(),
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
                            
                            log::info!("✓ Chain reorganization complete: replaced {} blocks with {} new blocks (hash tiebreaker)", 
                                orphan_count, saved);
                        } else {
                            log::info!("Local branch has equal difficulty, equal length, and lower/equal hash - keeping it");
                            anyhow::bail!("Local branch wins tiebreaker, no reorg needed");
                        }
                    } else {
                        log::info!("Local branch has equal difficulty and length - keeping it");
                        anyhow::bail!("No clear winner, keeping local chain");
                    }
                } else {
                    log::info!("Local branch is longer with equal difficulty - keeping it");
                    anyhow::bail!("Local branch is longer, no reorg needed");
                }
            } else {
                log::info!("Local branch has higher cumulative difficulty - keeping it");
                anyhow::bail!("Local branch has higher difficulty, no reorg needed");
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
    datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    swarm: std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
    fork_config: modal_observer::ForkConfig,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    initial_difficulty: Option<u128>,
    miner_hash_func: Option<String>,
    miner_hash_params: Option<serde_json::Value>,
    mining_delay_ms: Option<u64>,
    epoch_transition_tx: Option<tokio::sync::broadcast::Sender<u64>>,
) -> Result<MiningOutcome> {
    use modal_miner::{Blockchain, ChainConfig};
    
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

    // Create ChainConfig with custom initial_difficulty and mining_delay
    let chain_config = ChainConfig {
        initial_difficulty: initial_difficulty.unwrap_or(1000),
        target_block_time_secs: 60,
        mining_delay_ms,
    };

    // Load blockchain from datastore using the load_or_create_with_fork_config API
    let mut chain = Blockchain::load_or_create_with_fork_config(
        chain_config,
        peer_id.to_string(),
        datastore.clone(),
        fork_config,
    ).await?;
    
    log::info!("Loaded chain with {} blocks (height: {})", 
        chain.blocks.len(),
        chain.height());
    
    // Determine hash function and params with precedence:
    // 1. Genesis contract (if available)
    // 2. Node config
    // 3. Default "randomx"
    let (final_hash_func, final_hash_params) = {
        // Try to load from genesis contract if network has one
        let datastore_guard = datastore.lock().await;
        let genesis_params = datastore_guard.get_string("/network/genesis_contract_id").await.ok()
            .flatten()
            .and_then(|contract_id| {
                futures::executor::block_on(async {
                    datastore_guard.load_network_parameters_from_contract(&contract_id).await.ok()
                })
            });
        drop(datastore_guard);
        
        if let Some(params) = genesis_params {
            log::info!("Using miner hash configuration from genesis contract: {}", params.miner_hash_func);
            (params.miner_hash_func, params.mining_hash_params)
        } else {
            let hash_func = miner_hash_func.unwrap_or_else(|| {
                log::info!("Using default miner hash function: randomx");
                "randomx".to_string()
            });
            log::info!("Using miner hash configuration from node config: {}", hash_func);
            (hash_func, miner_hash_params)
        }
    };
    
    // Set RandomX parameters if using randomx and params are provided
    if final_hash_func == "randomx" && final_hash_params.is_some() {
        modal_common::hash_tax::set_randomx_params_from_json(final_hash_params.as_ref());
        log::info!("Set custom RandomX parameters for mining");
    }
    
    // Create a new Miner with the determined hash function
    let custom_miner = modal_miner::Miner::new(modal_miner::MinerConfig {
        max_tries: None,
        hash_func_name: Some(final_hash_func.leak()), // Convert String to &'static str
        mining_delay_ms: chain.config.mining_delay_ms,
    });
    chain.miner = custom_miner;
    
    // Check if we're trying to mine a block that already exists
    if index < chain.height() + 1 && index < chain.blocks.len() as u64 {
        // Block already exists in the chain, skip it
        log::warn!("⏭️  Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
        return Ok(MiningOutcome::Skipped);
    }
    
    // Verify we're mining the correct next block
    let expected_next = chain.height() + 1;
    if index != expected_next {
        log::error!("Index mismatch: expected to mine block {}, but was asked to mine block {}", expected_next, index);
        return Err(anyhow::anyhow!("Index mismatch: chain expects block {} but trying to mine {}", expected_next, index));
    }
    
    log::info!("Chain ready for mining. Height: {}, Mining next index: {}", chain.height(), index);
    
    // Mine the next block with persistence and fork choice
    let miner_number = rand::random::<u64>();
    let (mined_block, mining_stats) = chain.mine_block_with_persistence(
        nominated_peer_id.clone(), 
        miner_number
    ).await?;
    
    // Update mining metrics if we got stats
    if let Some(stats) = mining_stats {
        let mut metrics = mining_metrics.write().await;
        metrics.record_block_mined(stats.attempts as u64, stats.duration_secs);
        
        let avg_hashrate = metrics.average_hashrate();
        let total_blocks = metrics.blocks_mined;
        
        log::info!("⛏️  Block {} mined: {} attempts in {:.2}s, instant: {:.2} H/s", 
            index, stats.attempts, stats.duration_secs, stats.hashrate());
        log::info!("📊 Miner Stats: avg_hashrate={:.2} H/s, total_blocks={}, total_hashes={}", 
            avg_hashrate, total_blocks, metrics.total_hashes);
    }
    
    // Verify the mined block has the expected index
    if mined_block.header.index != index {
        log::error!("Mined block index mismatch: expected {}, got {}", index, mined_block.header.index);
        return Err(anyhow::anyhow!("Mined block index mismatch"));
    }

    // Convert to MinerBlock for gossip
    let miner_block = MinerBlock::new_canonical(
        mined_block.header.hash.clone(),
        index,
        index / 40, // Assuming 40 blocks per epoch
        mined_block.header.timestamp.timestamp(),
        mined_block.header.previous_hash.clone(),
        mined_block.header.data_hash.clone(),
        mined_block.header.nonce,
        mined_block.header.difficulty,
        mined_block.data.nominated_peer_id.clone(),
        mined_block.data.miner_number,
    );

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
        
        // Broadcast epoch transition for hybrid consensus coordination
        if let Some(tx) = epoch_transition_tx {
            if let Err(e) = tx.send(miner_block.epoch) {
                log::debug!("No receivers for epoch transition: {}", e);
            } else {
                log::info!("📡 Broadcasted epoch {} transition for validator coordination", miner_block.epoch);
            }
        }
    }

    Ok(MiningOutcome::Mined)
}

/// Efficiently find the common ancestor between local and remote chains using binary search
/// 
/// This function uses the `/data/miner_block/find_ancestor` route to iteratively find
/// the highest block index where both chains agree, using an exponential search followed
/// by binary search for O(log n) complexity.
/// 
/// # Arguments
/// * `swarm` - The swarm for making requests
/// * `peer_addr` - The peer address to query
/// * `datastore` - Local datastore to get our chain
/// 
/// # Returns
/// * `Ok(Some(index))` - The index of the common ancestor
/// * `Ok(None)` - No common ancestor found (different genesis)
/// * `Err(_)` - Error during the search
/// Returns (ancestor_index, peer_chain_length, peer_cumulative_difficulty)
pub async fn find_common_ancestor_efficient(
    swarm: &std::sync::Arc<tokio::sync::Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    datastore: &std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    reqres_response_txs: &std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
) -> Result<(Option<u64>, u64, u128)> {
    use libp2p::multiaddr::Multiaddr;
    
    log::info!("🔍 Finding common ancestor with peer using efficient binary search");
    
    // Load our local canonical chain
    let local_blocks = {
        let ds = datastore.lock().await;
        MinerBlock::find_all_canonical(&ds).await?
    };
    
    // Parse peer address early so we can use it in both branches
    let ma: Multiaddr = peer_addr.parse()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Invalid peer address - missing PeerID");
    };
    
    if local_blocks.is_empty() {
        log::info!("Local chain is empty, no common ancestor");
        
        // Still need to get the peer's chain info
        // Request chain info from peer
        let request = crate::reqres::Request {
            path: "/data/miner_block/chain_info".to_string(),
            data: None,
        };
        
        let request_id = {
            let mut swarm_lock = swarm.lock().await;
            swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
        };
        
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            wait_for_reqres_response(reqres_response_txs, request_id)
        ).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                log::warn!("Failed to get chain info from peer: {}", e);
                return Ok((None, 0, 0));
            }
            Err(_) => {
                log::warn!("Timeout waiting for chain info from peer");
                return Ok((None, 0, 0));
            }
        };
        
        if !response.ok {
            log::warn!("Peer returned error for chain info request");
            return Ok((None, 0, 0));
        }
        
        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in chain info response"))?;
        let peer_chain_length = data.get("chain_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let peer_cumulative_difficulty = data.get("cumulative_difficulty")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);
        
        log::info!("Peer chain: {} blocks, cumulative difficulty: {}", peer_chain_length, peer_cumulative_difficulty);
        
        return Ok((None, peer_chain_length, peer_cumulative_difficulty));
    }
    
    let local_chain_length = local_blocks.len() as u64;
    log::debug!("Local chain length: {}", local_chain_length);
    
    // Step 1: Exponential search to find an upper bound
    // Check blocks at indices: [tip, tip-1, tip-2, tip-4, tip-8, tip-16, ...]
    let mut checkpoints = Vec::new();
    let mut step = 0;
    
    loop {
        let index = if step == 0 {
            local_chain_length.saturating_sub(1)
        } else if step == 1 {
            local_chain_length.saturating_sub(2)
        } else {
            local_chain_length.saturating_sub(1 << step)
        };
        
        if index >= local_chain_length {
            break;
        }
        
        if let Some(block) = local_blocks.iter().find(|b| b.index == index) {
            checkpoints.push((block.index, block.hash.clone()));
        }
        
        if index == 0 {
            break;
        }
        
        step += 1;
    }
    
    log::debug!("Phase 1: Exponential search with {} checkpoints", checkpoints.len());
    
    // Make the initial request
    let request = crate::reqres::Request {
        path: "/data/miner_block/find_ancestor".to_string(),
        data: Some(serde_json::json!({
            "check_points": checkpoints.iter().map(|(idx, hash)| {
                serde_json::json!({
                    "index": idx,
                    "hash": hash
                })
            }).collect::<Vec<_>>()
        })),
    };
    
    let request_id = {
        let mut swarm_lock = swarm.lock().await;
        swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
    };
    
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        wait_for_reqres_response(&reqres_response_txs, request_id)
    ).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return Err(e),
        Err(_) => anyhow::bail!("Timeout waiting for find_ancestor response"),
    };
    
    if !response.ok {
        anyhow::bail!("Peer returned error: {:?}", response.errors);
    }
    
    let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in response"))?;
    let highest_match = data.get("highest_match").and_then(|v| v.as_u64());
    let remote_chain_length = data.get("chain_length").and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing chain_length in response"))?;
    let remote_cumulative_difficulty = data.get("cumulative_difficulty")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid cumulative_difficulty in response"))?;
    
    log::info!("Remote chain length: {}, cumulative difficulty: {}, Initial highest match: {:?}", 
        remote_chain_length, remote_cumulative_difficulty, highest_match);
    
    // If no match at all, chains have no common ancestor
    if highest_match.is_none() {
        log::warn!("No common blocks found - chains have completely diverged (different genesis?)");
        return Ok((None, remote_chain_length, remote_cumulative_difficulty));
    }
    
    let mut highest_match_idx = highest_match.unwrap();
    
    // Step 2: Binary search to find the exact divergence point
    // We need to search between highest_match and the next checkpoint that didn't match
    let matches = data.get("matches").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing matches in response"))?;
    
    // Find the range to binary search
    let mut search_low = highest_match_idx;
    let mut search_high = local_chain_length - 1;
    
    // Find the first non-matching index that's higher than highest_match
    for match_info in matches {
        let idx = match_info.get("index").and_then(|v| v.as_u64()).unwrap();
        let matches_val = match_info.get("matches").and_then(|v| v.as_bool()).unwrap();
        
        if !matches_val && idx > highest_match_idx && idx < search_high {
            search_high = idx;
        }
    }
    
    log::debug!("Phase 2: Binary search between {} and {}", search_low, search_high);
    
    // Binary search to narrow down the exact divergence point
    while search_low < search_high && search_high - search_low > 1 {
        let mid = (search_low + search_high) / 2;
        
        // Check if we have a block at mid index
        let mid_block = match local_blocks.iter().find(|b| b.index == mid) {
            Some(block) => block,
            None => {
                // If we don't have this block locally, adjust search range
                search_high = mid;
                continue;
            }
        };
        
        log::debug!("Binary search: checking index {} (range: {} to {})", mid, search_low, search_high);
        
        // Query just this one checkpoint
        let request = crate::reqres::Request {
            path: "/data/miner_block/find_ancestor".to_string(),
            data: Some(serde_json::json!({
                "check_points": [{
                    "index": mid,
                    "hash": mid_block.hash
                }]
            })),
        };
        
        let request_id = {
            let mut swarm_lock = swarm.lock().await;
            swarm_lock.behaviour_mut().reqres.send_request(&target_peer_id, request)
        };
        
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            wait_for_reqres_response(&reqres_response_txs, request_id)
        ).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => return Err(e),
            Err(_) => anyhow::bail!("Timeout during binary search"),
        };
        
        if !response.ok {
            anyhow::bail!("Peer error during binary search: {:?}", response.errors);
        }
        
        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in binary search response"))?;
        let matches_array = data.get("matches").and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing matches"))?;
        
        if let Some(match_info) = matches_array.first() {
            let matches_val = match_info.get("matches").and_then(|v| v.as_bool()).unwrap_or(false);
            
            if matches_val {
                // This block matches, search higher
                search_low = mid;
                highest_match_idx = mid;
                log::debug!("Block {} matches, searching higher", mid);
            } else {
                // This block doesn't match, search lower
                search_high = mid;
                log::debug!("Block {} doesn't match, searching lower", mid);
            }
        }
    }
    
    log::info!("✅ Found common ancestor at block index {}", highest_match_idx);
    Ok((Some(highest_match_idx), remote_chain_length, remote_cumulative_difficulty))
}


