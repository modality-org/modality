use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::node::Node;
use modal_datastore::models::miner::MinerBlock;
use modal_datastore::Model;
use libp2p::{Multiaddr, PeerId};

#[derive(Debug, Parser)]
#[command(about = "Sync blockchain from network peers")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Stop syncing this many blocks before the highest known block height
    #[clap(long, default_value = "10")]
    block_height_minus: u64,

    /// Maximum number of peers to attempt sync from
    #[clap(long, default_value = "5")]
    max_peers: usize,

    /// Timeout per peer sync attempt in seconds
    #[clap(long, default_value = "30")]
    timeout_secs: u64,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    let mut node = Node::from_config(config.clone()).await?;
    
    println!("╭─────────────────────────────────────────────────────────────╮");
    println!("│  Modal Node Sync                                            │");
    println!("╰─────────────────────────────────────────────────────────────╯");
    println!();
    println!("🆔  Node: {}", node.peerid);
    println!();
    
    // Setup node
    node.setup(&config).await?;
    
    // Get current local chain state
    let local_chain_info = {
        let ds = node.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        let height = canonical_blocks.last().map(|b| b.index).unwrap_or(0);
        let count = canonical_blocks.len();
        (height, count)
    };
    
    println!("📊  Local Chain State");
    println!("    Height: {}", local_chain_info.0);
    println!("    Total Blocks: {}", local_chain_info.1);
    println!();
    
    // Check if we have any bootstrappers/peers
    if node.bootstrappers.is_empty() {
        println!("⚠️   No bootstrapper nodes configured");
        println!("    Please configure bootstrappers in your node config to sync from peers");
        return Ok(());
    }
    
    println!("🌐  Attempting to sync from {} peer(s)", node.bootstrappers.len().min(opts.max_peers));
    println!("    Stop at: {} blocks before chain tip", opts.block_height_minus);
    println!();
    
    let start_time = Instant::now();
    let mut synced_from_any_peer = false;
    let mut highest_peer_height = 0u64;
    let mut peers_attempted = 0;
    
    // Clone bootstrappers to avoid borrow issues
    let bootstrappers = node.bootstrappers.clone();
    
    // Try to sync from each bootstrapper
    for bootstrapper in bootstrappers.iter().take(opts.max_peers) {
        if peers_attempted >= opts.max_peers {
            break;
        }
        peers_attempted += 1;
        
        let addr_str = bootstrapper.to_string();
        println!("🔄  Peer {}/{}: {}", peers_attempted, opts.max_peers, addr_str);
        
        // Extract peer ID from multiaddr
        use libp2p::multiaddr::Protocol;
        let peer_id = bootstrapper.iter()
            .find_map(|proto| {
                if let Protocol::P2p(id) = proto {
                    Some(id)
                } else {
                    None
                }
            });
        
        let Some(peer_id) = peer_id else {
            println!("    ❌ Invalid peer address (no peer ID)");
            println!();
            continue;
        };
        
        // Request chain info from peer
        match sync_from_peer(
            &mut node,
            peer_id,
            bootstrapper.clone(),
            opts.block_height_minus,
            opts.timeout_secs,
        ).await {
            Ok(sync_result) => {
                println!("    ✅ Synced {} blocks from this peer", sync_result.blocks_synced);
                if let Some(peer_height) = sync_result.peer_height {
                    println!("    📏 Peer chain height: {}", peer_height);
                    highest_peer_height = highest_peer_height.max(peer_height);
                }
                if sync_result.blocks_synced > 0 {
                    synced_from_any_peer = true;
                }
                println!();
            }
            Err(e) => {
                println!("    ❌ Failed: {}", e);
                println!();
                continue;
            }
        }
    }
    
    let duration = start_time.elapsed();
    
    // Get final chain state
    let final_chain_info = {
        let ds = node.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        let height = canonical_blocks.last().map(|b| b.index).unwrap_or(0);
        let count = canonical_blocks.len();
        (height, count)
    };
    
    println!("╭─────────────────────────────────────────────────────────────╮");
    println!("│  Sync Summary                                               │");
    println!("╰─────────────────────────────────────────────────────────────╯");
    println!();
    println!("⏱️   Duration: {:.2}s", duration.as_secs_f64());
    println!("👥  Peers Attempted: {}", peers_attempted);
    println!();
    println!("📊  Chain State:");
    println!("    Before: {} blocks (height {})", local_chain_info.1, local_chain_info.0);
    println!("    After:  {} blocks (height {})", final_chain_info.1, final_chain_info.0);
    println!("    Added:  {} blocks", final_chain_info.1.saturating_sub(local_chain_info.1));
    println!();
    
    if highest_peer_height > 0 {
        let distance_from_tip = highest_peer_height.saturating_sub(final_chain_info.0);
        println!("🎯  Sync Status:");
        println!("    Highest Known Height: {}", highest_peer_height);
        println!("    Current Height: {}", final_chain_info.0);
        println!("    Distance from Tip: {} blocks", distance_from_tip);
        
        if distance_from_tip <= opts.block_height_minus {
            println!("    ✅ Within target range (--block-height-minus {})", opts.block_height_minus);
        } else {
            println!("    ⚠️  Still {} blocks behind target", distance_from_tip.saturating_sub(opts.block_height_minus));
        }
        println!();
    }
    
    if synced_from_any_peer {
        println!("✅  Sync completed successfully!");
    } else if peers_attempted == 0 {
        println!("⚠️   No peers available to sync from");
    } else {
        println!("⚠️   Could not sync from any peers");
    }
    
    Ok(())
}

struct SyncResult {
    blocks_synced: usize,
    peer_height: Option<u64>,
}

async fn sync_from_peer(
    node: &mut Node,
    peer_id: PeerId,
    peer_addr: Multiaddr,
    block_height_minus: u64,
    timeout_secs: u64,
) -> Result<SyncResult> {
    // Connect to peer with timeout
    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs / 2),
        node.connect_to_peer_multiaddr(peer_addr.clone())
    ).await;
    
    if let Err(_) = connect_result {
        anyhow::bail!("Connection timeout");
    }
    connect_result??;
    
    // Get peer's chain info first
    let chain_info_response = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs / 2),
        node.send_request(peer_id, "/data/miner_block/chain_info".to_string(), "{}".to_string())
    ).await;
    
    let chain_info = match chain_info_response {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Chain info request failed: {}", e);
        }
        Err(_) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Chain info request timeout");
        }
    };
    
    // Extract peer's chain height
    let peer_height = chain_info.data
        .as_ref()
        .and_then(|d| d.get("chain_height"))
        .and_then(|h| h.as_u64());
    
    let Some(peer_height) = peer_height else {
        let _ = node.disconnect_from_peer_id(peer_id).await;
        anyhow::bail!("Could not determine peer chain height");
    };
    
    // Calculate sync target (peer_height - block_height_minus)
    let target_height = peer_height.saturating_sub(block_height_minus);
    
    // Get our current height
    let our_height = {
        let ds = node.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        canonical_blocks.last().map(|b| b.index).unwrap_or(0)
    };
    
    // If we're already at or past the target, no need to sync
    if our_height >= target_height {
        let _ = node.disconnect_from_peer_id(peer_id).await;
        return Ok(SyncResult {
            blocks_synced: 0,
            peer_height: Some(peer_height),
        });
    }
    
    // Request blocks from our_height + 1 to target_height
    let from_index = our_height + 1;
    let to_index = target_height;
    
    let sync_response = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        node.send_request(
            peer_id,
            "/data/miner_block/range".to_string(),
            serde_json::json!({
                "from_index": from_index,
                "to_index": to_index,
            }).to_string()
        )
    ).await;
    
    let response = match sync_response {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Sync request failed: {}", e);
        }
        Err(_) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Sync request timeout");
        }
    };
    
    // Persist blocks
    let blocks_synced = if let Some(ref data) = response.data {
        if let Some(blocks) = data.get("blocks").and_then(|b| b.as_array()) {
            let mut persisted = 0;
            let mut ds = node.datastore.lock().await;
            
            for block_value in blocks {
                if let Ok(block) = serde_json::from_value::<MinerBlock>(block_value.clone()) {
                    // Save as canonical
                    if let Ok(_) = block.save(&mut ds).await {
                        persisted += 1;
                    }
                }
            }
            persisted
        } else {
            0
        }
    } else {
        0
    };
    
    // Disconnect from peer
    let _ = node.disconnect_from_peer_id(peer_id).await;
    
    Ok(SyncResult {
        blocks_synced,
        peer_height: Some(peer_height),
    })
}

