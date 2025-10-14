use anyhow::Result;
use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/range
/// Returns canonical miner blocks in a range (from_index..=to_index)
/// Useful for syncing blocks incrementally
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    let from_index = data.get("from_index").and_then(|v| v.as_u64());
    let to_index = data.get("to_index").and_then(|v| v.as_u64());
    
    match (from_index, to_index) {
        (Some(from), Some(to)) => {
            if from > to {
                return Ok(Response {
                    ok: false,
                    data: None,
                    errors: Some(serde_json::json!({"error": "from_index must be <= to_index"})),
                });
            }
            
            // Load all canonical blocks and filter by range
            match MinerBlock::find_all_canonical(datastore).await {
                Ok(all_blocks) => {
                    let blocks: Vec<_> = all_blocks
                        .into_iter()
                        .filter(|b| b.index >= from && b.index <= to)
                        .collect();
                    
                    Ok(Response {
                        ok: true,
                        data: Some(serde_json::json!({
                            "from_index": from,
                            "to_index": to,
                            "blocks": blocks,
                            "count": blocks.len(),
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

