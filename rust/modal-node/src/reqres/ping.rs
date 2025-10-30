use anyhow::Result;
use serde_json;

use crate::reqres::Response;

pub async fn handler(data: Option<serde_json::Value>) -> Result<Response> {
    log::info!("REQ /ping {:?}", data);
    let response = Response {
        ok: true,
        data: Some(data.unwrap()),
        errors: None
    };
    Ok(response)
}