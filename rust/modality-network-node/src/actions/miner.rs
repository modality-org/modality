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
    node.wait_for_connections().await?;

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
        let ds_guard = datastore.lock().await;
        use modality_network_mining::persistence::BlockchainPersistence;
        
        let loaded_blocks = ds_guard.load_canonical_blocks().await?;
        drop(ds_guard); // Release the lock
        
        if loaded_blocks.is_empty() {
            // No existing blocks, create genesis
            Blockchain::new(ChainConfig::default(), peer_id.to_string())
        } else {
            // Reconstruct blockchain from loaded blocks
            // Start with a fresh chain and replace genesis, then add remaining blocks
            let mut chain = Blockchain::new(ChainConfig::default(), peer_id.to_string());
            
            // Replace the auto-generated genesis with the loaded one
            chain.blocks.clear();
            chain.blocks.push(loaded_blocks[0].clone());
            
            // Add all subsequent blocks (add_block will handle block_index updates)
            for block in loaded_blocks.into_iter().skip(1) {
                chain.add_block(block)?;
            }
            
            chain
        }
    };
    
    // Check if we're trying to mine a block that already exists
    if index <= chain.height() {
        // Block already exists in the chain, return it
        log::warn!("Block {} already exists in chain (height: {}), skipping mining", index, chain.height());
        return Ok(());
    }
    
    // Mine the next block (difficulty will be calculated based on loaded blockchain state)
    let miner_number = rand::random::<u64>();
    let mined_block = chain.mine_block(nominated_peer_id.clone(), miner_number)?;

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

    // Save to datastore
    {
        let mut ds = datastore.lock().await;
        miner_block.save(&mut ds).await?;
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

    log::info!("Mined block {} with hash {}", miner_block.index, miner_block.hash);

    Ok(())
}


