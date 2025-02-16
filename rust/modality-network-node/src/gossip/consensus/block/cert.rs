use anyhow::Result;
use serde_json;
use tokio::sync::mpsc;

use modality_network_datastore::NetworkDatastore;
use modality_network_consensus::communication::Message as ConsensusMessage;

pub const TOPIC: &str = "/consensus/block/cert";

pub async fn handler(data: String, datastore: &NetworkDatastore, _consensus_tx: mpsc::Sender<ConsensusMessage>) -> Result<()> {
  let block_data = serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
  log::info!("{:?}", block_data);
  log::info!("current_round: {:?}", datastore.get_current_round().await);
  //   await node.services.local.consensus.onReceiveBlockCert(obj);
  Ok(())
}
