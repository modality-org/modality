use anyhow::Result;
use serde_json;

use modal_datastore::NetworkDatastore;

use crate::reqres::Response;

pub async fn handler(data: Option<serde_json::Value>, _datastore: &NetworkDatastore) -> Result<Response> {
    log::info!("REQ /data/block/head {:?}", data);
    let response = Response {
        ok: true,
        data: None,
        errors: None
    };
    Ok(response)
}