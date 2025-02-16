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
    let running = Arc::new(AtomicBool::new(true));
    let running_shared: Arc<AtomicBool> = running.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl-C, shutting down...");
        running_shared.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");


    let tick_interval: Duration = Duration::from_secs(15);
    let mut tick = futures_timer::Delay::new(tick_interval);

    gossip::add_sequencer_event_listeners(node).await?;

    node.run().await?;

    Ok(())
}
