use anyhow::Result;
use anyhow::anyhow;
use modal_datastore::Model;
use serde_json;
use tokio::sync::mpsc;

use modal_datastore::NetworkDatastore;
use modal_datastore::models::ValidatorBlock;
use modal_validator_consensus::communication::Message as ConsensusMessage;

pub const TOPIC: &str = "/consensus/block/cert";

pub async fn handler(data: String, _datastore: &NetworkDatastore, consensus_tx: mpsc::Sender<ConsensusMessage>) -> Result<()> {
  // log::info!("current_round: {:?}", datastore.get_current_round().await);

  let block_data = serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
  let block = ValidatorBlock::from_json_string(&data.clone())?;
  let from = block_data.get("peer_id")
    .ok_or_else(|| anyhow!("Missing peer_id field"))?
    .as_str()
    .ok_or_else(|| anyhow!("peer_id is not a string"))?;

  let msg = ConsensusMessage::CertifiedValidatorBlock {
    from: from.to_string(),
    to: String::new(),
    block: block,
  };
  consensus_tx.send(msg).await?;
  Ok(())
}