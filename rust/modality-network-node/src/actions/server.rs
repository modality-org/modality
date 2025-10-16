use anyhow::Result;

use crate::node::Node;
use crate::gossip;

pub async fn run(node: &mut Node) -> Result<()> {
    gossip::add_sequencer_event_listeners(node).await?;

    node.start_networking().await?;
    node.start_autoupgrade().await?;
    node.wait_for_connections().await?;
    node.start_consensus().await?;

    node.wait_for_shutdown().await?;

    Ok(())
}
