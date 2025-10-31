use anyhow::{Context, Result};
use libp2p::PeerId;
use modal_sequencer_consensus::narwhal::{SyncRequest, SyncResponse};
use crate::node::Node;

/// Make a DAG sync request to a peer
pub async fn sync_request(
    node: &mut Node,
    peer_id: PeerId,
    sync_req: SyncRequest,
) -> Result<SyncResponse> {
    // Serialize the sync request
    let request_data = serde_json::to_string(&sync_req)
        .context("Failed to serialize sync request")?;
    
    // Send request to peer
    let response = node
        .send_request(peer_id, "/dag/sync".to_string(), request_data)
        .await
        .context("Failed to send sync request")?;
    
    // Check if request succeeded
    if !response.ok {
        let error_msg = response
            .errors
            .and_then(|e| e.get("error").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_else(|| "Unknown error".to_string());
        anyhow::bail!("Sync request failed: {}", error_msg);
    }
    
    // Deserialize the response
    let Some(data) = response.data else {
        anyhow::bail!("Sync response missing data");
    };
    
    let sync_response: SyncResponse = serde_json::from_value(data)
        .context("Failed to deserialize sync response")?;
    
    Ok(sync_response)
}

/// Example: Sync DAG with a peer
/// 
/// ```no_run
/// use modal_node::actions::dag_sync;
/// use modal_sequencer_consensus::narwhal::SyncRequest;
/// 
/// async fn example(node: &mut Node, peer_id: PeerId) -> Result<()> {
///     // Get peer's highest round
///     let req = SyncRequest::highest_round();
///     let resp = dag_sync::sync_request(node, peer_id, req).await?;
///     
///     match resp {
///         SyncResponse::HighestRound { round } => {
///             println!("Peer's highest round: {}", round);
///         }
///         _ => {}
///     }
///     
///     Ok(())
/// }
/// ```
pub async fn sync_with_peer(
    node: &mut Node,
    peer_id: PeerId,
) -> Result<()> {
    // Get peer's highest round
    let highest_req = SyncRequest::highest_round();
    let highest_resp = sync_request(node, peer_id, highest_req).await?;
    
    let peer_highest = match highest_resp {
        SyncResponse::HighestRound { round } => round,
        _ => anyhow::bail!("Unexpected response to highest round request"),
    };
    
    log::info!("Peer {} highest round: {}", peer_id, peer_highest);
    
    // TODO: Implement full sync logic
    // 1. Determine our highest round
    // 2. Request certificates in batches
    // 3. Insert into our DAG
    // 4. Verify and process
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: Full integration tests would require a running node
    // These are placeholder tests showing the API
    
    #[test]
    fn test_sync_request_serialization() {
        let req = SyncRequest::highest_round();
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("GetHighestRound"));
    }
}

