use anyhow::Result;
use crate::node::Node;

pub const TOPIC: &str = "/consensus/block/cert";

pub async fn handler(node: &mut Node, data: String) -> Result<()> {
  //   const text = new TextDecoder().decode(event.detail.data);
  //   const obj = SafeJSON.parse(text);
  //   await node.services.local.consensus.onReceiveBlockCert(obj);
  Ok(())
}
