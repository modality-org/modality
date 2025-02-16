use anyhow::Result;
use serde_json;

use modality_network_datastore::NetworkDatastore;

pub const TOPIC: &str = "/consensus/block/draft";

pub async fn handler(data: String, datastore: &NetworkDatastore) -> Result<()> {
  let block_data = serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
  log::info!("{:?}", block_data);
  log::info!("current_round: {:?}", datastore.get_current_round().await);
  // await node.services.local.consensus.onReceiveBlockDraft(block_data);
  Ok(())
}