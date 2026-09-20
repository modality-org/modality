use anyhow::Result;
use crate::reqres::Response;
use crate::inspection::{InspectionLevel, InspectionData, NodeStatus, DatastoreInfo};
use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;
use serde_json;

/// Handler for inspection requests
pub async fn handler(
    data: Option<serde_json::Value>,
    datastore_manager: &DatastoreManager,
) -> Result<Response> {
    let level = if let Some(ref data) = data {
        if let Some(level_str) = data.get("level").and_then(|v| v.as_str()) {
            level_str.parse::<InspectionLevel>().unwrap_or(InspectionLevel::Basic)
        } else {
            InspectionLevel::Basic
        }
    } else {
        InspectionLevel::Basic
    };

    let inspection_data = get_datastore_inspection(datastore_manager, level).await?;

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(inspection_data)?),
        errors: None,
    })
}

/// Get inspection data from datastore
pub async fn get_datastore_inspection(
    datastore_manager: &DatastoreManager,
    level: InspectionLevel,
) -> Result<InspectionData> {
    let mut data = InspectionData::new_basic("unknown".to_string(), NodeStatus::Running);
    
    if InspectionData::should_include_datastore(level) {
        let blocks = MinerBlock::find_all_canonical_multi(datastore_manager).await?;
        
        let block_range = if !blocks.is_empty() {
            Some((blocks.first().unwrap().index, blocks.last().unwrap().index))
        } else {
            None
        };
        
        let chain_tip_height = blocks.last().map(|b| b.index);
        let chain_tip_hash = blocks.last().map(|b| b.hash.clone());
        
        let mut epochs_set = std::collections::HashSet::new();
        for block in &blocks {
            epochs_set.insert(block.epoch);
        }
        
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
        None => {
            if let Some(requester) = requesting_peer_id {
                requester == node_peer_id
            } else {
                true
            }
        }
        Some(whitelist) if whitelist.is_empty() => {
            requesting_peer_id.is_none()
        }
        Some(whitelist) => {
            if let Some(requester) = requesting_peer_id {
                whitelist.contains(&requester.to_string()) || requester == node_peer_id
            } else {
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
        assert!(is_authorized(Some("12D3KooWNode"), node_peer, None));
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, None));
        assert!(is_authorized(None, node_peer, None));
    }

    #[test]
    fn test_authorization_empty_whitelist() {
        let node_peer = "12D3KooWNode";
        let whitelist = vec![];
        assert!(!is_authorized(Some("12D3KooWNode"), node_peer, Some(&whitelist)));
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, Some(&whitelist)));
        assert!(is_authorized(None, node_peer, Some(&whitelist)));
    }

    #[test]
    fn test_authorization_with_whitelist() {
        let node_peer = "12D3KooWNode";
        let whitelist = vec![
            "12D3KooWAllowed1".to_string(),
            "12D3KooWAllowed2".to_string(),
        ];
        assert!(is_authorized(Some("12D3KooWNode"), node_peer, Some(&whitelist)));
        assert!(is_authorized(Some("12D3KooWAllowed1"), node_peer, Some(&whitelist)));
        assert!(!is_authorized(Some("12D3KooWOther"), node_peer, Some(&whitelist)));
        assert!(is_authorized(None, node_peer, Some(&whitelist)));
    }
}
