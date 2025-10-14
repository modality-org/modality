use anyhow::Result;
use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::MinerBlock;
use modality_network_datastore::Model;
use crate::reqres::Response;

/// Get a miner block by hash
pub mod get;

/// Get all canonical miner blocks
pub mod list_canonical;

/// Get miner blocks by epoch
pub mod by_epoch;

/// Get miner block range by indices
pub mod range;

/// Handler for GET /data/miner_block/:hash
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
                    errors: Some(serde_json::json!({"error": "Miner block not found"})),
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

