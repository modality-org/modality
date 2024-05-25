use libp2p::request_response;

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
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub data: serde_json::Value,
}
