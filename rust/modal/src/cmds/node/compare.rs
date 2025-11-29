use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::node::Node;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::miner::MinerBlock;
use libp2p::{PeerId, Multiaddr};
use libp2p::multiaddr::Protocol;

#[derive(Debug, Parser)]
#[command(about = "Compare local chain with a remote peer's chain")]
pub struct Opts {
    /// Peer ID to compare with (can be a peer ID or multiaddr)
    #[clap(name = "PEER")]
    pub peer: String,
    
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,
    
    /// Node directory containing config.json
    #[clap(long)]
    pub dir: Option<PathBuf>,
    
    /// Timeout in seconds for network requests
    #[clap(long, default_value = "30")]
    pub timeout_secs: u64,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    
    // Open local datastore in read-only mode
    let storage_path = config.storage_path.as_ref()
        .context("No storage_path in config")?;
    let datastore = NetworkDatastore::create_in_directory_readonly(&storage_path)?;
    
    // Get local chain info
    let local_blocks = MinerBlock::find_all_canonical(&datastore).await?;
    let local_orphans = MinerBlock::find_all_orphaned(&datastore).await?;
    let local_chain_length = local_blocks.len() as u64;
    
    // Calculate local cumulative difficulty
    let mut local_cumulative_difficulty: u128 = 0;
    for block in &local_blocks {
        if let Ok(diff) = block.get_difficulty_u128() {
            local_cumulative_difficulty += diff;
        }
    }
    
    // Parse peer address (could be peer ID or multiaddr)
    let (peer_id, peer_addr) = parse_peer_address(&opts.peer, &config)?;
    
    println!("🔍 Comparing chains with peer {}", peer_id);
    println!();
    
    // Create Node instance for network communication
    let mut node = Node::from_config(config.clone()).await?;
    node.setup(&config).await?;
    
    // Compare with peer
    let comparison = compare_with_peer(
        &mut node,
        peer_id,
        peer_addr,
        &local_blocks,
        opts.timeout_secs
    ).await?;
    
    // Display comparison results
    println!("📊 Chain Comparison");
    println!("==================");
    println!();
    println!("Local Chain:");
    println!("  Length: {} blocks", local_chain_length);
    println!("  Orphans: {} blocks", local_orphans.len());
    println!("  Cumulative Difficulty: {}", local_cumulative_difficulty);
    if let Some(tip) = local_blocks.last() {
        println!("  Tip Hash: {}", tip.hash);
        println!("  Tip Index: {}", tip.index);
    }
    println!();
    
    println!("Remote Chain:");
    println!("  Length: {} blocks", comparison.remote_chain_length);
    println!("  Cumulative Difficulty: {}", comparison.remote_cumulative_difficulty);
    if let Some(hash) = &comparison.remote_tip_hash {
        println!("  Tip Hash: {}", hash);
    }
    println!();
    
    if let Some(common_ancestor) = comparison.common_ancestor_index {
        println!("✓ Common Ancestor: Block {}", common_ancestor);
        
        // Show the hash at common ancestor
        if let Some(ancestor_block) = local_blocks.iter().find(|b| b.index == common_ancestor) {
            println!("  Hash: {}", ancestor_block.hash);
        }
        
        // Show divergence info
        let local_diverged_blocks = local_chain_length.saturating_sub(common_ancestor + 1);
        let remote_diverged_blocks = comparison.remote_chain_length.saturating_sub(common_ancestor + 1);
        
        if local_diverged_blocks > 0 || remote_diverged_blocks > 0 {
            println!();
            println!("⚠️  FORK DETECTED");
            println!("  Local diverged: {} blocks (from {} to {})", 
                local_diverged_blocks,
                common_ancestor + 1,
                local_chain_length - 1
            );
            println!("  Remote diverged: {} blocks (from {} to {})",
                remote_diverged_blocks,
                common_ancestor + 1,
                comparison.remote_chain_length - 1
            );
            
            // Show which chain is ahead
            if local_cumulative_difficulty > comparison.remote_cumulative_difficulty {
                println!();
                println!("✓ Local chain is heavier (ahead by {} difficulty)",
                    local_cumulative_difficulty - comparison.remote_cumulative_difficulty);
            } else if comparison.remote_cumulative_difficulty > local_cumulative_difficulty {
                println!();
                println!("⚠️  Remote chain is heavier (ahead by {} difficulty)",
                    comparison.remote_cumulative_difficulty - local_cumulative_difficulty);
                println!("   Consider syncing to adopt the heavier chain:");
                println!("   modal node sync");
            } else {
                println!();
                println!("✓ Chains have equal cumulative difficulty");
            }
        } else {
            println!("✓ Chains are in sync");
        }
    } else {
        println!("❌ No common ancestor found - chains completely diverged from genesis");
    }
    
    Ok(())
}

struct ChainComparison {
    remote_chain_length: u64,
    remote_cumulative_difficulty: u128,
    remote_tip_hash: Option<String>,
    common_ancestor_index: Option<u64>,
}

fn parse_peer_address(
    peer_str: &str,
    config: &modal_node::config::Config,
) -> Result<(PeerId, Multiaddr)> {
    // Try parsing as multiaddr first
    if let Ok(addr) = peer_str.parse::<Multiaddr>() {
        // Extract peer ID from multiaddr
        let peer_id = addr.iter()
            .find_map(|proto| {
                if let Protocol::P2p(id) = proto {
                    Some(id)
                } else {
                    None
                }
            })
            .context("Multiaddr does not contain a peer ID")?;
        return Ok((peer_id, addr));
    }
    
    // Try parsing as peer ID
    if let Ok(peer_id) = peer_str.parse::<PeerId>() {
        // Look for this peer in bootstrappers
        if let Some(ref bootstrappers) = config.bootstrappers {
            for addr in bootstrappers {
                let addr_peer_id = addr.iter()
                    .find_map(|proto| {
                        if let Protocol::P2p(id) = proto {
                            Some(id)
                        } else {
                            None
                        }
                    });
                
                if addr_peer_id == Some(peer_id) {
                    return Ok((peer_id, addr.clone()));
                }
            }
        }
        
        anyhow::bail!("Peer ID not found in bootstrappers. Please provide a full multiaddr.");
    }
    
    anyhow::bail!("Invalid peer address. Please provide either:\n  - A full multiaddr (e.g. /ip4/1.2.3.4/tcp/4040/ws/p2p/12D3...)\n  - A peer ID found in your config's bootstrappers")
}

async fn compare_with_peer(
    node: &mut Node,
    peer_id: PeerId,
    peer_addr: Multiaddr,
    local_blocks: &[MinerBlock],
    timeout_secs: u64,
) -> Result<ChainComparison> {
    // Connect to peer
    println!("🔗 Connecting to peer...");
    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs / 2),
        node.connect_to_peer_multiaddr(peer_addr.clone())
    ).await;
    
    if let Err(_) = connect_result {
        anyhow::bail!("Connection timeout");
    }
    connect_result??;
    println!("   Connected!");
    println!();
    
    // Get peer's chain info
    println!("📡 Requesting chain info...");
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
    
    // Extract remote chain info
    let remote_chain_length = chain_info.data
        .as_ref()
        .and_then(|d| d.get("chain_height"))
        .and_then(|h| h.as_u64())
        .context("Could not determine peer chain height")?;
    
    let remote_cumulative_difficulty = chain_info.data
        .as_ref()
        .and_then(|d| d.get("cumulative_difficulty"))
        .and_then(|h| h.as_str())
        .and_then(|s| s.parse::<u128>().ok())
        .context("Could not determine peer cumulative difficulty")?;
    
    let remote_tip_hash = chain_info.data
        .as_ref()
        .and_then(|d| d.get("tip_hash"))
        .and_then(|h| h.as_str())
        .map(|s| s.to_string());
    
    // Build checkpoints for find_ancestor query
    println!("🔎 Finding common ancestor...");
    let local_chain_length = local_blocks.len() as u64;
    let mut checkpoints = Vec::new();
    let mut step = 0;
    
    // Exponential backoff: [tip, tip-1, tip-2, tip-4, tip-8, ...]
    loop {
        let index = if step == 0 {
            local_chain_length.saturating_sub(1)
        } else if step == 1 {
            local_chain_length.saturating_sub(2)
        } else {
            local_chain_length.saturating_sub(1 << step)
        };
        
        if index >= local_chain_length { break; }
        
        if let Some(block) = local_blocks.iter().find(|b| b.index == index) {
            checkpoints.push(serde_json::json!({
                "index": block.index,
                "hash": block.hash
            }));
        }
        
        if index == 0 { break; }
        step += 1;
    }
    
    // Send find_ancestor request
    let ancestor_response = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        node.send_request(
            peer_id,
            "/data/miner_block/find_ancestor".to_string(),
            serde_json::json!({
                "check_points": checkpoints
            }).to_string()
        )
    ).await;
    
    let ancestor = match ancestor_response {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Find ancestor request failed: {}", e);
        }
        Err(_) => {
            let _ = node.disconnect_from_peer_id(peer_id).await;
            anyhow::bail!("Find ancestor request timeout");
        }
    };
    
    let common_ancestor_index = ancestor.data
        .as_ref()
        .and_then(|d| d.get("highest_match"))
        .and_then(|h| h.as_u64());
    
    // Disconnect from peer
    let _ = node.disconnect_from_peer_id(peer_id).await;
    
    Ok(ChainComparison {
        remote_chain_length,
        remote_cumulative_difficulty,
        remote_tip_hash,
        common_ancestor_index,
    })
}

