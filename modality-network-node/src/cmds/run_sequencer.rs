use crate::config_file;
use crate::identity_utils;
use crate::swarm;

use anyhow::{Context, Result};
use clap::Parser;
use futures::future::{select, Either};
use futures::StreamExt;
use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
use std::borrow::Borrow;
use std::time::Duration;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long, default_value = "./config.json")]
    config: std::path::PathBuf,

    #[clap(long)]
    keypair: Option<std::path::PathBuf>,

    #[clap(long, default_value = "/ip4/0.0.0.0/tcp/0/ws")]
    listen: String,

    #[clap(long)]
    storage: Option<std::path::PathBuf>,

    #[clap(long, default_value = "15")]
    tick_interval: u64,
}

// pub async fn run(arg_matches: &clap::ArgMatches) -> Result<()> {
pub async fn run_sequencer(opts: &Opts) -> Result<()> {
    log::info!("Config: {:?}", opts.config);
 
    let config =
        config_file::read_or_create_config(&opts.config).context("Failed to read config")?;

    log::info!("Config: {:?}", config);

    let config_keypair = config.keypair.unwrap();

    let node_keypair = identity_utils::identity_from_private_key(
      config_keypair.private_key.unwrap_or_default().borrow(),
    )
    .await?;
    // .context("Failed to read identity")?;
    log::info!("Node keypair: {:?}", node_keypair.public());
    let node_peer_id = PeerId::from(node_keypair.public());

    let mut swarm = swarm::create_swarm(node_keypair).await?;

    let listen_ma = config.listen.unwrap_or(opts.listen.clone()).parse::<Multiaddr>().unwrap();
    swarm.listen_on(listen_ma.clone()).expect("");

    let tick_interval: Duration = Duration::from_secs(opts.tick_interval);
    let mut tick = futures_timer::Delay::new(tick_interval);

    loop {
        match select(swarm.next(), &mut tick).await {
            Either::Left((event, _)) => match event.unwrap() {
                SwarmEvent::NewListenAddr { address, .. } => {
                    let address_with_p2p = address.clone().with(libp2p::multiaddr::Protocol::P2p(node_peer_id));
                    log::info!("Listening on {address_with_p2p:?}")
                }
                event => {
                    log::debug!("Other type of event: {:?}", event);
                }
            },
            Either::Right(_) => {
                log::debug!("tick");
                tick = futures_timer::Delay::new(tick_interval);
            }
        }
    }
}
