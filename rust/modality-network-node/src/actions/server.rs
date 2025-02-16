use crate::node::Node;

use anyhow::Result;
use futures::future::{select, Either};
use futures::prelude::*;

use std::time::Duration;

use libp2p::gossipsub;
// use libp2p::kad;
use libp2p::request_response;
use libp2p::swarm::SwarmEvent;
use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::gossip;

pub async fn run(node: &mut Node) -> Result<()> {
    gossip::add_sequencer_event_listeners(node).await?;

    node.run().await?;

    Ok(())
}
