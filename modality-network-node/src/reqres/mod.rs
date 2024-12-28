use anyhow::Result;
use libp2p::request_response;
mod consensus;
mod ping;
mod data;
use data as reqres_data;

#[allow(dead_code)]
pub const PROTOCOL: &str = "/modality-network/reqres/0.0.1";
#[allow(dead_code)]
pub const PROTOCOL_VERSION: &str = "0.0.1";
#[allow(dead_code)]
pub const PROTOCOL_PREFIX: &str = "modality-network";
#[allow(dead_code)]
pub const PROTOCOL_NAME: &str = "reqres";

pub type Behaviour = request_response::json::Behaviour::<Request, Response>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub path: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub ok: bool,
    pub data: Option<serde_json::Value>,
    pub errors: Option<serde_json::Value>
}

pub async fn handle_request(req: Request) -> Result<Response> {
    log::info!("Handling request: {:?}", req);
    let path = req.path;
    let data = req.data.unwrap_or_default();
    let response = match path.as_str() {
        "/ping" => {
            ping::handler(Some(data.clone())).await?
        },
        "/data/block/head" => {
            reqres_data::block::head::handler(Some(data.clone())).await?
        }
        "/data/block/body" => {
            reqres_data::block::body::handler(Some(data.clone())).await?
        }
        "/data/block/inclusions" => {
            reqres_data::block::inclusions::handler(Some(data.clone())).await?
        }
        "/consensus/status" => {
            consensus::status::handler(Some(data.clone())).await?
        }
        _ => {
            Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": "Unknown path"}))
            }
        }
    };
    log::info!("Response: {:?}", response);
    Ok(response)
}