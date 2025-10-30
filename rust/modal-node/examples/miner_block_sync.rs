/// Example: Miner Block Synchronization Between Nodes
/// 
/// This example demonstrates:
/// 1. Node 1 has some persisted miner blocks
/// 2. Node 2 connects to Node 1
/// 3. Node 2 syncs the miner blocks from Node 1 using the request-response protocol
/// 
/// Usage:
///   cargo run --example miner_block_sync

use anyhow::Result;
use futures::StreamExt;
use libp2p::{Multiaddr, Swarm};
use libp2p::swarm::SwarmEvent;
use modal_datastore::models::MinerBlock;
use modal_datastore::Model;
use modal_node::config::Config;
use modal_node::reqres::{Request, Response};
use modal_node::swarm;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("\n=== Miner Block Sync Example ===\n");
    
    // ========== STEP 1: Setup datastores and create blocks ==========
    println!("📦 Setting up datastores...");
    
    let temp_dir1 = tempfile::tempdir()?;
    let storage_path1 = temp_dir1.path().join("node1_data");
    let mut datastore1 = modal_datastore::NetworkDatastore::create_in_directory(&storage_path1)?;
    
    let temp_dir2 = tempfile::tempdir()?;
    let storage_path2 = temp_dir2.path().join("node2_data");
    let mut datastore2 = modal_datastore::NetworkDatastore::create_in_directory(&storage_path2)?;
    
    println!("  ✓ Datastores created\n");
    
    // ========== STEP 2: Create and persist miner blocks in Node 1's datastore ==========
    println!("⛏  Creating miner blocks in Node 1's datastore...");
    
    for i in 0..=10 {
        let block = MinerBlock::new_canonical(
            format!("block_hash_{:03}", i),
            i,
            i / 40, // epoch
            1234567890 + (i as i64 * 60),
            if i == 0 { "0".to_string() } else { format!("block_hash_{:03}", i - 1) },
            format!("data_hash_{:03}", i),
            10000 + (i as u128),
            1000,
            format!("QmMiner{:02}", i % 5),
            1000 + i,
        );
        
        block.save(&datastore1).await?;
        
        if i == 0 {
            println!("  ✓ Genesis block: hash={}", block.hash);
        } else if i <= 3 || i >= 8 {
            let hash_display = if block.hash.len() >= 16 { &block.hash[..16] } else { &block.hash };
            let peer_display = if block.nominated_peer_id.len() >= 13 { &block.nominated_peer_id[..13] } else { &block.nominated_peer_id };
            println!("  ✓ Block {}: hash={}, peer={}", i, hash_display, peer_display);
        } else if i == 4 {
            println!("  ...");
        }
    }
    
    println!("  ✓ Total blocks created: 11 (0-10)\n");
    
    // ========== STEP 3: Create network nodes ==========
    println!("🌐 Creating network nodes...");
    
    // Create Node 1 keypair and swarm
    let node1_keypair = libp2p::identity::Keypair::generate_ed25519();
    let node1_peer_id = node1_keypair.public().to_peer_id();
    let mut node1_swarm = swarm::create_swarm(node1_keypair.clone()).await?;
    
    // Node 1 listen address (use 0 for random port assignment to avoid conflicts)
    let listen_addr1: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    node1_swarm.listen_on(listen_addr1.clone())?;
    
    println!("  ✓ Node 1:");
    println!("    Peer ID: {}", node1_peer_id);
    println!("    Listening: {}", listen_addr1);
    
    // Create Node 2 keypair and swarm
    let node2_keypair = libp2p::identity::Keypair::generate_ed25519();
    let node2_peer_id = node2_keypair.public().to_peer_id();
    let mut node2_swarm = swarm::create_swarm(node2_keypair.clone()).await?;
    
    // Node 2 listen address (use 0 for random port assignment to avoid conflicts)
    let listen_addr2: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    node2_swarm.listen_on(listen_addr2.clone())?;
    
    println!("  ✓ Node 2:");
    println!("    Peer ID: {}", node2_peer_id);
    println!("    Listening: {}", listen_addr2);
    
    // Wait for listeners to be ready and get actual addresses
    let actual_listen1 = wait_for_listener(&mut node1_swarm).await?;
    let actual_listen2 = wait_for_listener(&mut node2_swarm).await?;
    
    println!("  Actual address 1: {}", actual_listen1);
    println!("  Actual address 2: {}", actual_listen2);
    
    println!("\n🔗 Connecting Node 2 to Node 1...");
    
    // Connect Node 2 to Node 1 using actual listening address
    let mut node1_addr = actual_listen1.clone();
    node1_addr.push(libp2p::multiaddr::Protocol::P2p(node1_peer_id));
    node2_swarm.dial(node1_addr.clone())?;
    
    // Wait for connection (process events on both swarms)
    wait_for_connection_both(&mut node1_swarm, &mut node2_swarm, node1_peer_id).await?;
    
    println!("  ✓ Nodes connected\n");
    
    // ========== STEP 4: Sync blocks from Node 1 to Node 2 ==========
    println!("🔄 Syncing miner blocks from Node 1 to Node 2...\n");
    
    // Request all canonical blocks
    let request = Request {
        path: "/data/miner_block/canonical".to_string(),
        data: None,
    };
    
    println!("  📤 Node 2 requesting canonical blocks from Node 1...");
    let request_id = node2_swarm.behaviour_mut().reqres.send_request(&node1_peer_id, request);
    println!("  ⏳ Request ID: {:?}", request_id);
    
    // Handle request-response between nodes
    let response = handle_sync(
        &mut node1_swarm,
        &mut node2_swarm,
        &mut datastore1,
        &mut datastore2,
        request_id,
    ).await?;
    
    if response.ok {
        if let Some(data) = response.data {
            let blocks: Vec<serde_json::Value> = serde_json::from_value(
                data.get("blocks").unwrap().clone()
            )?;
            
            println!("\n  ✓ Received {} blocks from Node 1\n", blocks.len());
            println!("📊 Synced Blocks:");
            
            for block in &blocks {
                let index = block.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let hash = block.get("hash").and_then(|v| v.as_str()).unwrap_or("unknown");
                let peer_id = block.get("nominated_peer_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let epoch = block.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0);
                
                let hash_display = if hash.len() >= 16 { &hash[..16] } else { hash };
                let peer_display = if peer_id.len() >= 10 { &peer_id[..10] } else { peer_id };
                
                println!("  Block {:2}: epoch={}, peer={}, hash={}",
                    index, epoch, peer_display, hash_display);
            }
            
            println!("\n✅ Sync completed successfully!");
        }
    } else {
        println!("  ❌ Sync failed: {:?}", response.errors);
    }
    
    // ========== STEP 5: Query specific epoch ==========
    println!("\n🔍 Querying epoch 0 blocks from Node 1...");
    
    let epoch_request = Request {
        path: "/data/miner_block/epoch".to_string(),
        data: Some(serde_json::json!({"epoch": 0})),
    };
    
    let request_id = node2_swarm.behaviour_mut().reqres.send_request(&node1_peer_id, epoch_request);
    
    let response = handle_sync(
        &mut node1_swarm,
        &mut node2_swarm,
        &mut datastore1,
        &mut datastore2,
        request_id,
    ).await?;
    
    if response.ok {
        if let Some(data) = response.data {
            let count = data.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("  ✓ Epoch 0 contains {} blocks\n", count);
        }
    }
    
    // ========== STEP 6: Request block range ==========
    println!("🔍 Requesting blocks 3-7 from Node 1...");
    
    let range_request = Request {
        path: "/data/miner_block/range".to_string(),
        data: Some(serde_json::json!({
            "from_index": 3,
            "to_index": 7
        })),
    };
    
    let request_id = node2_swarm.behaviour_mut().reqres.send_request(&node1_peer_id, range_request);
    
    let response = handle_sync(
        &mut node1_swarm,
        &mut node2_swarm,
        &mut datastore1,
        &mut datastore2,
        request_id,
    ).await?;
    
    if response.ok {
        if let Some(data) = response.data {
            let blocks: Vec<serde_json::Value> = serde_json::from_value(
                data.get("blocks").unwrap().clone()
            )?;
            
            println!("  ✓ Received {} blocks in range 3-7:", blocks.len());
            for block in &blocks {
                let index = block.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let hash = block.get("hash").and_then(|v| v.as_str()).unwrap_or("unknown");
                let hash_display = if hash.len() >= 16 { &hash[..16] } else { hash };
                println!("    Block {}: hash={}", index, hash_display);
            }
        }
    }
    
    println!("\n✅ Miner block sync example completed!");
    println!("\n📋 Summary:");
    println!("  • Node 1 had 11 persisted miner blocks");
    println!("  • Node 2 synced all blocks from Node 1");
    println!("  • Demonstrated epoch and range queries");
    println!("  • All communication via TCP transport");
    println!("  • Request-response protocol: /modality-network/reqres/0.0.1");
    println!("\nℹ️  Note: For WebSocket, use addresses like:");
    println!("     /ip4/0.0.0.0/tcp/10001/ws");
    
    Ok(())
}

/// Wait for listener to be ready and return the actual address
async fn wait_for_listener(swarm: &mut Swarm<swarm::NodeBehaviour>) -> Result<Multiaddr> {
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
                return Ok(address);
            }
            _ => {}
        }
    }
}

/// Wait for connection to be established (process events on both swarms)
async fn wait_for_connection_both(
    swarm1: &mut Swarm<swarm::NodeBehaviour>,
    swarm2: &mut Swarm<swarm::NodeBehaviour>,
    target_peer: libp2p::PeerId,
) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();
    
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Connection timeout");
        }
        
        tokio::select! {
            event1 = swarm1.select_next_some() => {
                if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event1 {
                    if peer_id == target_peer {
                        log::info!("Node 1 connected to {}", peer_id);
                    }
                }
            }
            event2 = swarm2.select_next_some() => {
                match event2 {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if peer_id == target_peer {
                            log::info!("Node 2 connected to {}", peer_id);
                            return Ok(());
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        log::error!("Connection error to {:?}: {}", peer_id, error);
                        anyhow::bail!("Connection error");
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Handle sync between two nodes
async fn handle_sync(
    node1_swarm: &mut Swarm<swarm::NodeBehaviour>,
    node2_swarm: &mut Swarm<swarm::NodeBehaviour>,
    datastore1: &mut modal_datastore::NetworkDatastore,
    datastore2: &mut modal_datastore::NetworkDatastore,
    request_id: libp2p::request_response::OutboundRequestId,
) -> Result<Response> {
    use libp2p::request_response;
    use modal_sequencer_consensus::communication::Message as ConsensusMessage;
    
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();
    
    let (tx, _rx) = tokio::sync::mpsc::channel::<ConsensusMessage>(100);
    
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Sync timeout");
        }
        
        tokio::select! {
            event1 = node1_swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                    request_response::Event::Message {
                        message: request_response::Message::Request { request, channel, .. },
                        ..
                    }
                )) = event1 {
                    // Handle request on Node 1
                    let response = modal_node::reqres::handle_request(request, datastore1, tx.clone()).await?;
                    node1_swarm.behaviour_mut().reqres.send_response(channel, response).ok();
                }
            }
            event2 = node2_swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                    request_response::Event::Message {
                        message: request_response::Message::Response { request_id: rid, response },
                        ..
                    }
                )) = event2 {
                    if rid == request_id {
                        return Ok(response);
                    }
                }
            }
        }
    }
}
