use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for POST /data/miner_block/debug_index
/// Returns ALL blocks at a specific index (canonical, orphaned, and pending)
/// Useful for debugging chain integrity issues
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    let index = match data.get("index").and_then(|v| v.as_u64()) {
        Some(idx) => idx,
        None => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": "Missing 'index' parameter"})),
            });
        }
    };
    
    // Get all blocks at this index
    let all_at_index = MinerBlock::find_by_index(datastore, index).await?;
    
    // Also get the canonical block specifically
    let canonical = MinerBlock::find_canonical_by_index(datastore, index).await?;
    
    // If we have a next block, get its prev_hash to verify chain linkage
    let next_blocks = MinerBlock::find_by_index(datastore, index + 1).await?;
    let canonical_next = next_blocks.iter().find(|b| b.is_canonical);
    
    // Build diagnostic info
    let mut block_info: Vec<serde_json::Value> = Vec::new();
    for block in &all_at_index {
        block_info.push(serde_json::json!({
            "hash": block.hash,
            "previous_hash": block.previous_hash,
            "is_canonical": block.is_canonical,
            "is_orphaned": block.is_orphaned,
            "orphan_reason": block.orphan_reason,
        }));
    }
    
    let mut result = serde_json::json!({
        "index": index,
        "total_blocks_at_index": all_at_index.len(),
        "blocks": block_info,
    });
    
    if let Some(canonical_block) = canonical {
        result["canonical_hash"] = serde_json::json!(canonical_block.hash);
    }
    
    if let Some(next_canonical) = canonical_next {
        result["next_block_index"] = serde_json::json!(next_canonical.index);
        result["next_block_prev_hash"] = serde_json::json!(next_canonical.previous_hash);
        
        // Check if there's a matching block at current index
        let matching_block = all_at_index.iter().find(|b| b.hash == next_canonical.previous_hash);
        if let Some(matching) = matching_block {
            result["matching_block_for_next"] = serde_json::json!({
                "hash": matching.hash,
                "is_canonical": matching.is_canonical,
                "is_orphaned": matching.is_orphaned,
            });
        } else {
            result["chain_integrity_issue"] = serde_json::json!(
                format!("No block at index {} has hash matching next block's prev_hash {}", 
                    index, &next_canonical.previous_hash[..20])
            );
        }
    }
    
    log::info!("Debug index {}: {} blocks found", index, all_at_index.len());
    
    Ok(Response {
        ok: true,
        data: Some(result),
        errors: None,
    })
}

