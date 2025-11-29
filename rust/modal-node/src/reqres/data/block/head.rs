use anyhow::Result;
use serde_json;

use modal_datastore::DatastoreManager;

use crate::reqres::Response;

pub async fn handler(data: Option<serde_json::Value>, _datastore_manager: &DatastoreManager) -> Result<Response> {
    log::info!("REQ /data/block/head {:?}", data);
    let response = Response {
        ok: true,
        data: None,
        errors: None
    };
    Ok(response)
}
