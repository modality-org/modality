use anyhow::Result;
use crate::reqres::{Response};
use serde_json;

pub async fn handler(data: Option<serde_json::Value>) -> Result<Response> {
    log::info!("REQ /ping {:?}", data);
    let response = Response {
        ok: true,
        data: Some(data.unwrap()),
        errors: None
    };
    Ok(response)
}