use anyhow::Result;
use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for POST /data/miner_block/find_ancestor
pub async fn handler(
    data: Option<serde_json::Value>, 
    datastore_manager: &DatastoreManager,
) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    let check_points = match data.get("check_points").and_then(|v| v.as_array()) {
        Some(arr) => {
            let mut points = Vec::new();
            for item in arr {
                if let (Some(index), Some(hash)) = (
                    item.get("index").and_then(|v| v.as_u64()),
                    item.get("hash").and_then(|v| v.as_str())
                ) {
                    points.push((index, hash.to_string()));
                } else {
                    return Ok(Response {
                        ok: false,
                        data: None,
                        errors: Some(serde_json::json!({
                            "error": "Invalid check_point format"
                        })),
                    });
                }
            }
            points
        }
        None => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({
                    "error": "Missing 'check_points' parameter"
                })),
            });
        }
    };
    
    let canonical_blocks = match MinerBlock::find_all_canonical_multi(datastore_manager).await {
        Ok(blocks) => blocks,
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": format!("Failed to load canonical blocks: {}", e)})),
            });
        }
    };
    
    let chain_length = canonical_blocks.len() as u64;
    
    let cumulative_difficulty = match MinerBlock::calculate_cumulative_difficulty(&canonical_blocks) {
        Ok(diff) => diff.to_string(),
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({
                    "error": format!("Failed to calculate cumulative difficulty: {}", e)
                })),
            });
        }
    };
    
    let mut index_to_hash = std::collections::HashMap::new();
    for block in &canonical_blocks {
        index_to_hash.insert(block.index, block.hash.clone());
    }
    
    let mut matches = Vec::new();
    let mut highest_match: Option<u64> = None;
    
    for (index, hash) in check_points {
        let matches_local = match index_to_hash.get(&index) {
            Some(local_hash) => {
                let is_match = local_hash == &hash;
                if is_match && (highest_match.is_none() || highest_match.unwrap() < index) {
                    highest_match = Some(index);
                }
                is_match
            }
            None => false
        };
        
        matches.push(serde_json::json!({
            "index": index,
            "hash": hash,
            "matches": matches_local,
        }));
    }
    
    Ok(Response {
        ok: true,
        data: Some(serde_json::json!({
            "chain_length": chain_length,
            "matches": matches,
            "highest_match": highest_match,
            "cumulative_difficulty": cumulative_difficulty,
        })),
        errors: None,
    })
}
