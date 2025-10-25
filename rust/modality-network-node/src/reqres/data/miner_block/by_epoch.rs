use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/epoch/:epoch
/// Returns all canonical miner blocks for a specific epoch
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    if let Some(epoch) = data.get("epoch").and_then(|v| v.as_u64()) {
        match MinerBlock::find_canonical_by_epoch(datastore, epoch).await {
            Ok(blocks) => {
                Ok(Response {
                    ok: true,
                    data: Some(serde_json::json!({
                        "epoch": epoch,
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
    } else {
        Ok(Response {
            ok: false,
            data: None,
            errors: Some(serde_json::json!({"error": "Missing 'epoch' parameter"})),
        })
    }
}

