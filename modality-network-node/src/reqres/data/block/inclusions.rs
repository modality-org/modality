use anyhow::Result;
use crate::reqres::{Response};
use serde_json;

pub async fn handler(data: Option<serde_json::Value>) -> Result<Response> {
    log::info!("REQ /data/block/inclusions {:?}", data);
    let response = Response {
        ok: true,
        data: None,
        errors: None
    };
    Ok(response)
}