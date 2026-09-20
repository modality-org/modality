use anyhow::Result;
use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for GET /data/miner_block/canonical
/// Returns all canonical miner blocks sorted by index
pub async fn handler(
    _data: Option<serde_json::Value>, 
    datastore_manager: &DatastoreManager,
) -> Result<Response> {
    match MinerBlock::find_all_canonical_multi(datastore_manager).await {
        Ok(blocks) => {
            Ok(Response {
                ok: true,
                data: Some(serde_json::json!({
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
