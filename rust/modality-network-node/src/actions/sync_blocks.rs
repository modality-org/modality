use crate::reqres;
use crate::node::Node;
use anyhow::Result;
use libp2p::multiaddr::Multiaddr;
use modal_datastore::models::MinerBlock;
use modal_datastore::Model;

/// Sync blocks from a remote node with optional persistence
pub async fn run(
    node: &mut Node,
    target: String,
    path: String,
    data: String,
    persist: bool,
) -> Result<SyncResult> {
    let ma = target.parse::<Multiaddr>()?;
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
    };

    // Connect to peer
    node.connect_to_peer_multiaddr(ma.clone()).await?;

    // Send request
    let response = node.send_request(target_peer_id, path.clone(), data.clone()).await?;
    
    if !response.ok {
        node.disconnect_from_peer_id(target_peer_id).await?;
        return Ok(SyncResult {
            response,
            persisted_count: None,
            skipped_count: None,
        });
    }

    // Persist blocks if requested
    let (persisted_count, skipped_count) = if persist {
        if let Some(ref data) = response.data {
            persist_blocks(data, &node.datastore).await?
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    // Disconnect
    node.disconnect_from_peer_id(target_peer_id).await?;

    Ok(SyncResult {
        response,
        persisted_count: if persist { Some(persisted_count) } else { None },
        skipped_count: if persist { Some(skipped_count) } else { None },
    })
}

/// Result of a sync operation
pub struct SyncResult {
    pub response: reqres::Response,
    pub persisted_count: Option<usize>,
    pub skipped_count: Option<usize>,
}

/// Persist synced blocks to the datastore
async fn persist_blocks(
    data: &serde_json::Value,
    datastore: &std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
) -> Result<(usize, usize)> {
    let blocks = data
        .get("blocks")
        .and_then(|b| b.as_array())
        .ok_or_else(|| anyhow::anyhow!("No blocks in response"))?;

    let mut ds = datastore.lock().await;
    let mut saved_count = 0;
    let mut skipped_count = 0;

    for block_json in blocks {
        // Deserialize JSON to MinerBlock
        let block: MinerBlock = serde_json::from_value(block_json.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize block: {}", e))?;

        // Check if block already exists
        match MinerBlock::find_by_hash(&*ds, &block.hash).await {
            Ok(Some(_existing)) => {
                // Block already exists, skip
                skipped_count += 1;
                log::debug!("Block {} already exists, skipping", block.hash);
            }
            Ok(None) => {
                // Block doesn't exist, save it
                block
                    .save(&*ds)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to save block {}: {}", block.hash, e))?;
                saved_count += 1;
                log::debug!("Saved block {} to datastore", block.hash);
            }
            Err(e) => {
                log::warn!("Error checking block {}: {}", block.hash, e);
                // Try to save anyway
                block
                    .save(&*ds)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to save block {}: {}", block.hash, e))?;
                saved_count += 1;
            }
        }
    }

    if saved_count > 0 {
        log::info!("✓ Persisted {} blocks to datastore", saved_count);
    }
    if skipped_count > 0 {
        log::info!("Skipped {} blocks (already in datastore)", skipped_count);
    }

    Ok((saved_count, skipped_count))
}

