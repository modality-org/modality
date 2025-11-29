use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/range
/// Returns canonical miner blocks in a range (from_index..=to_index)
/// Useful for syncing blocks incrementally
/// Default limit is 50 blocks per request to avoid response size issues
/// Client can specify a max_chunk_size (capped at 1000 blocks)
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    let from_index = data.get("from_index").and_then(|v| v.as_u64());
    let to_index = data.get("to_index").and_then(|v| v.as_u64());
    let max_chunk_size = data.get("max_chunk_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(50);
    
    match (from_index, to_index) {
        (Some(from), Some(to)) => {
            if from > to {
                return Ok(Response {
                    ok: false,
                    data: None,
                    errors: Some(serde_json::json!({"error": "from_index must be <= to_index"})),
                });
            }
            
                    // Respect client's max_chunk_size, but cap at 1000 blocks per request to avoid extreme response sizes
                    let chunk_size = std::cmp::min(max_chunk_size, 1000);
                    let actual_to = std::cmp::min(to, from + chunk_size - 1);
            
            // Load all canonical blocks and filter by range
            match MinerBlock::find_all_canonical(datastore).await {
                Ok(all_blocks) => {
                    let blocks: Vec<_> = all_blocks
                        .into_iter()
                        .filter(|b| b.index >= from && b.index <= actual_to)
                        .collect();
                    
                    Ok(Response {
                        ok: true,
                        data: Some(serde_json::json!({
                            "from_index": from,
                            "to_index": actual_to,
                            "requested_to": to,
                            "blocks": blocks,
                            "count": blocks.len(),
                            "has_more": actual_to < to && !blocks.is_empty(),
                            "chunk_size": chunk_size,
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
        _ => {
            Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": "Missing 'from_index' or 'to_index' parameter"})),
            })
        }
    }
}

