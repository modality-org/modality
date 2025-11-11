use anyhow::Result;
use libp2p::request_response;
mod consensus;
mod ping;
mod data;
mod dag;
mod contract;
use data as reqres_data;
use tokio::sync::mpsc;

use modal_datastore::NetworkDatastore;
use modal_validator_consensus::communication::Message as ConsensusMessage;

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

pub async fn handle_request(req: Request, datastore: &mut NetworkDatastore, consensus_tx: mpsc::Sender<ConsensusMessage>) -> Result<Response> {
    log::info!("Handling request: {:?}", req);
    let path = req.path;
    let data = req.data.unwrap_or_default();
    let response = match path.as_str() {
        "/ping" => {
            ping::handler(Some(data.clone())).await?
        },
        "/data/block" => {
            reqres_data::block::handler(Some(data.clone()), datastore).await?
        }
        "/data/block/head" => {
            reqres_data::block::head::handler(Some(data.clone()), datastore).await?
        }
        "/data/block/body" => {
            reqres_data::block::body::handler(Some(data.clone()), datastore).await?
        }
        "/data/block/inclusions" => {
            reqres_data::block::inclusions::handler(Some(data.clone()), datastore).await?
        }
        "/consensus/status" => {
            consensus::status::handler(Some(data.clone()), datastore).await?
        }
        "/consensus/block/ack" => {
            consensus::block::ack::handler(Some(data.clone()), datastore, consensus_tx).await?
        }
        "/data/miner_block/get" => {
            reqres_data::miner_block::get::handler(Some(data.clone()), datastore).await?
        }
        "/data/miner_block/canonical" => {
            reqres_data::miner_block::list_canonical::handler(Some(data.clone()), datastore).await?
        }
        "/data/miner_block/epoch" => {
            reqres_data::miner_block::by_epoch::handler(Some(data.clone()), datastore).await?
        }
        "/data/miner_block/range" => {
            reqres_data::miner_block::range::handler(Some(data.clone()), datastore).await?
        }
        "/data/miner_block/chain_info" => {
            reqres_data::miner_block::chain_info::handler(Some(data.clone()), datastore).await?
        }
        "/data/miner_block/find_ancestor" => {
            reqres_data::miner_block::find_ancestor::handler(Some(data.clone()), datastore).await?
        }
        "/dag/sync" => {
            dag::sync::handler(Some(data.clone()), datastore).await?
        }
        "/contract/submit" => {
            contract::submit::handler(Some(data.clone()), datastore, consensus_tx.clone()).await?
        }
        "/contract/push" => {
            contract::push::handler(Some(data.clone()), datastore, consensus_tx.clone()).await?
        }
        "/contract/pull" => {
            contract::pull::handler(Some(data.clone()), datastore, consensus_tx.clone()).await?
        }
        "/contract/list" => {
            contract::list::handler(Some(data.clone()), datastore, consensus_tx).await?
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