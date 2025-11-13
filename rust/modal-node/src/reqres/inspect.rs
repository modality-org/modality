use anyhow::Result;
use crate::reqres::Response;
use crate::inspection::{InspectionLevel, InspectionData, NodeStatus, DatastoreInfo};
use modal_datastore::NetworkDatastore;
use serde_json;

/// Handler for inspection requests
/// Note: This handler only returns datastore information since it runs in the reqres context
/// without full node state access. For full inspection, use the Node::get_inspection_data method.
pub async fn handler(
    data: Option<serde_json::Value>,
    datastore: &mut NetworkDatastore,
) -> Result<Response> {
    // Parse level from request data (though we only support datastore inspection via reqres)
    let level = if let Some(ref data) = data {
        if let Some(level_str) = data.get("level").and_then(|v| v.as_str()) {
            level_str.parse::<InspectionLevel>().unwrap_or(InspectionLevel::Basic)
        } else {
            InspectionLevel::Basic
        }
    } else {
        InspectionLevel::Basic
    };

    // Get datastore inspection data
    let inspection_data = get_datastore_inspection(datastore, level).await?;

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(inspection_data)?),
        errors: None,
    })
}

/// Get inspection data from datastore
pub async fn get_datastore_inspection(
    datastore: &mut NetworkDatastore,
    level: InspectionLevel,
) -> Result<InspectionData> {
    use modal_datastore::models::MinerBlock;
    
    // Note: peer_id will need to be filled in by the networking layer
    let mut data = InspectionData::new_basic("unknown".to_string(), NodeStatus::Running);
    
    // Always include datastore info for basic level and above
    if InspectionData::should_include_datastore(level) {
        let blocks = MinerBlock::find_all_canonical(datastore).await?;
        
        let block_range = if !blocks.is_empty() {
            Some((blocks.first().unwrap().index, blocks.last().unwrap().index))
        } else {
            None
        };
        
        let chain_tip_height = blocks.last().map(|b| b.index);
        let chain_tip_hash = blocks.last().map(|b| b.hash.clone());
        
        // Count unique epochs
        let mut epochs_set = std::collections::HashSet::new();
        for block in &blocks {
            epochs_set.insert(block.epoch);
        }
        
        // Count unique miners
        let mut miners_set = std::collections::HashSet::new();
        for block in &blocks {
            miners_set.insert(&block.nominated_peer_id);
        }
        
        data.datastore = Some(DatastoreInfo {
            total_blocks: blocks.len(),
            block_range,
            chain_tip_height,
            chain_tip_hash,
            epochs: Some(epochs_set.len()),
            unique_miners: Some(miners_set.len()),
        });
    }
    
    Ok(data)
}

/// Check if the requesting peer is authorized to inspect this node
pub fn is_authorized(
    requesting_peer_id: Option<&str>,
    node_peer_id: &str,
    inspect_whitelist: Option<&Vec<String>>,
) -> bool {
    match inspect_whitelist {
        // No whitelist configured: only allow self
        None => {
            if let Some(requester) = requesting_peer_id {
                requester == node_peer_id
            } else {
                // No peer ID provided (direct local access), allow
                true
            }
        }
        // Empty whitelist: reject all external requests
        Some(whitelist) if whitelist.is_empty() => {
            // Only allow if no peer ID (direct local access)
            requesting_peer_id.is_none()
        }
        // Populated whitelist: check if requester is in the list
        Some(whitelist) => {
            if let Some(requester) = requesting_peer_id {
                // Check if requester is in whitelist or is self
                whitelist.contains(&requester.to_string()) || requester == node_peer_id
            } else {
                // No peer ID provided (direct local access), allow
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_no_whitelist() {
        let node_peer = "12D3KooWNode";
        
        // Self should be allowed
        assert!(is_authorized(Some("12D3KooWNode"), node_peer, None));
        
        // Other peer should not be allowed
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, None));
        
        // Direct local access (no peer) should be allowed
        assert!(is_authorized(None, node_peer, None));
    }

    #[test]
    fn test_authorization_empty_whitelist() {
        let node_peer = "12D3KooWNode";
        let whitelist = vec![];
        
        // Even self should not be allowed with empty whitelist
        assert!(!is_authorized(Some("12D3KooWNode"), node_peer, Some(&whitelist)));
        
        // Other peer should not be allowed
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, Some(&whitelist)));
        
        // Direct local access should be allowed
        assert!(is_authorized(None, node_peer, Some(&whitelist)));
    }

    #[test]
    fn test_authorization_with_whitelist() {
        let node_peer = "12D3KooWNode";
        let whitelist = vec![
            "12D3KooWAllowed1".to_string(),
            "12D3KooWAllowed2".to_string(),
        ];
        
        // Self should always be allowed
        assert!(is_authorized(Some("12D3KooWNode"), node_peer, Some(&whitelist)));
        
        // Whitelisted peer should be allowed
        assert!(is_authorized(Some("12D3KooWAllowed1"), node_peer, Some(&whitelist)));
        assert!(is_authorized(Some("12D3KooWAllowed2"), node_peer, Some(&whitelist)));
        
        // Non-whitelisted peer should not be allowed
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, Some(&whitelist)));
        
        // Direct local access should be allowed
        assert!(is_authorized(None, node_peer, Some(&whitelist)));
    }
}

