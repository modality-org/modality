use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/chain_info
/// Returns chain information including cumulative difficulty and common ancestor
/// 
/// Request format:
/// {
///   "local_block_hashes": ["hash1", "hash2", ...], // Optional: for finding common ancestor
///   "include_blocks": true/false, // Optional: whether to include full block data
///   "from_index": u64, // Optional: if include_blocks=true, start from this index
/// }
/// 
/// Response format:
/// {
///   "cumulative_difficulty": string, // u128 as string
///   "chain_length": u64,
///   "common_ancestor_index": u64 or null,
///   "blocks": [...] or null // Only if include_blocks=true
/// }
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    // Get local block hashes for common ancestor detection
    let local_hashes = data.get("local_block_hashes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    
    let include_blocks = data.get("include_blocks")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let from_index = data.get("from_index")
        .and_then(|v| v.as_u64());
    
    // Load all canonical blocks
    match MinerBlock::find_all_canonical(datastore).await {
        Ok(all_blocks) => {
            if all_blocks.is_empty() {
                return Ok(Response {
                    ok: true,
                    data: Some(serde_json::json!({
                        "cumulative_difficulty": "0",
                        "chain_length": 0,
                        "common_ancestor_index": null,
                        "blocks": null,
                    })),
                    errors: None,
                });
            }
            
            // Calculate cumulative difficulty
            let cumulative_difficulty = match MinerBlock::calculate_cumulative_difficulty(&all_blocks) {
                Ok(diff) => diff,
                Err(e) => {
                    return Ok(Response {
                        ok: false,
                        data: None,
                        errors: Some(serde_json::json!({"error": format!("Failed to calculate cumulative difficulty: {}", e)})),
                    });
                }
            };
            
            let chain_length = all_blocks.len() as u64;
            
            // Find common ancestor by checking which of our blocks match the provided hashes
            let common_ancestor_index = if !local_hashes.is_empty() {
                all_blocks.iter()
                    .filter(|block| local_hashes.contains(&block.hash))
                    .map(|block| block.index)
                    .max()
            } else {
                None
            };
            
            // Optionally include blocks
            let blocks_data = if include_blocks {
                let start_index = from_index
                    .or(common_ancestor_index.map(|idx| idx + 1))
                    .unwrap_or(0);
                
                let filtered_blocks: Vec<_> = all_blocks
                    .into_iter()
                    .filter(|b| b.index >= start_index)
                    .collect();
                
                Some(serde_json::to_value(filtered_blocks)?)
            } else {
                None
            };
            
            Ok(Response {
                ok: true,
                data: Some(serde_json::json!({
                    "cumulative_difficulty": cumulative_difficulty.to_string(),
                    "chain_length": chain_length,
                    "common_ancestor_index": common_ancestor_index,
                    "blocks": blocks_data,
                })),
                errors: None,
            })
        }
        Err(e) => {
            Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": e.to_string()})),
            })
        }
    }
}

