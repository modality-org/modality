use anyhow::Result;
use crate::node::Node;
use serde_json;

pub const TOPIC: &str = "/consensus/block/draft";

pub async fn handler(node: &mut Node, data: String) -> Result<()> {
  let block_data = serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
  log::info!("{:?}", block_data);
  log::info!("current_round: {:?}", node.datastore.get_current_round().await);
  // await node.services.local.consensus.onReceiveBlockDraft(block_data);
  Ok(())
}