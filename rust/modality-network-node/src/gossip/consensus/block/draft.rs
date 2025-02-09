use anyhow::Result;
use crate::node::Node;

pub const TOPIC: &str = "/consensus/block/draft";

pub async fn handler(node: &mut Node, data: String) -> Result<()> {
  //   const block_data = SafeJSON.parse(text);
  //   await node.services.local.consensus.onReceiveBlockDraft(block_data);
  Ok(())
}