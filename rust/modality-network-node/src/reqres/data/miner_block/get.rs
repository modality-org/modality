use anyhow::Result;
use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::MinerBlock;
use modality_network_datastore::Model;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/get
/// Get a specific miner block by hash
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    if let Some(hash) = data.get("hash").and_then(|v| v.as_str()) {
        match MinerBlock::find_by_hash(datastore, hash).await {
            Ok(Some(block)) => {
                Ok(Response {
                    ok: true,
                    data: Some(serde_json::to_value(block)?),
                    errors: None,
                })
            }
            Ok(None) => {
                Ok(Response {
                    ok: false,
                    data: None,
                    errors: Some(serde_json::json!({"error": "Block not found"})),
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
            errors: Some(serde_json::json!({"error": "Missing 'hash' parameter"})),
        })
    }
}

