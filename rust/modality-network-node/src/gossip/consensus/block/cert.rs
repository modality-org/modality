use anyhow::Result;
use anyhow::anyhow;
use modality_network_datastore::Model;
use serde_json;
use tokio::sync::mpsc;

use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::Block;
use modality_network_consensus::communication::Message as ConsensusMessage;

pub const TOPIC: &str = "/consensus/block/cert";

pub async fn handler(data: String, _datastore: &NetworkDatastore, consensus_tx: mpsc::Sender<ConsensusMessage>) -> Result<()> {
  // log::info!("current_round: {:?}", datastore.get_current_round().await);

  let block_data = serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
  let block = Block::from_json_string(&data.clone())?;
  let from = block_data.get("peer_id")
    .ok_or_else(|| anyhow!("Missing peer_id field"))?
    .as_str()
    .ok_or_else(|| anyhow!("peer_id is not a string"))?;

  let msg = ConsensusMessage::CertifiedBlock {
    from: from.to_string(),
    to: String::new(),
    block: block,
  };
  consensus_tx.send(msg).await?;
  Ok(())
}