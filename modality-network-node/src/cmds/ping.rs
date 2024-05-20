use crate::config_file;
use crate::identity_utils;
use crate::swarm;

// use libp2p::swarm::behaviour::ConnectionEstablished;
use anyhow::{Context, Result};
use clap::Parser;
use libp2p::multiaddr::Multiaddr;
use std::borrow::Borrow;
use std::time::Instant;
use futures::prelude::*;
use libp2p::swarm::StreamProtocol;
use libp2p::swarm::SwarmEvent;
use rand::RngCore;
#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long, default_value = "./config.json")]
    config: std::path::PathBuf,

    #[clap(long)]
    keypair: Option<std::path::PathBuf>,

    #[clap(long)]
    storage: Option<std::path::PathBuf>,

    #[clap(long, default_value = "1")]
    times: i32,

    #[clap(long, default_value = "/ip4/0.0.0.0/tcp/0/ws")]
    target: String,
}

async fn send_random_ping(mut stream: libp2p::Stream) -> std::io::Result<()> {
    let num_bytes = 32;

    let mut bytes = vec![0; num_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);
    stream.write_all(&bytes).await?;

    let mut buf = vec![0; num_bytes];
    stream.read_exact(&mut buf).await?;

    if bytes != buf {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "incorrect echo"));
    }
    stream.close().await?;
    Ok(())
}

pub async fn run(opts: &Opts) -> Result<()> {
    let times_to_ping = *(&opts.times);

    let config = config_file::read_or_create_config(&opts.config).context("Failed to read config")?;
    let config_keypair = config.keypair.unwrap();
    let node_keypair = identity_utils::identity_from_private_key(
      config_keypair.private_key.unwrap_or_default().borrow(),
    )
    .await?;

    let ma = opts.target.clone().parse::<Multiaddr>().unwrap();

    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Provided address does not end in `/p2p`");
    };

    let mut swarm = swarm::create_swarm(node_keypair).await?;
    swarm.dial(ma.clone())?;
    let mut control = swarm.behaviour().stream.new_control();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if peer_id == target_peer_id {
                    log::info!("Connected to peer {:?}", peer_id);
                    break;
                }
            } 
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    log::error!("Failed to dial peer {:?}", peer_id);
                    log::error!("Error: {:?}", error);
                }
            }
            event => {
               log::debug!("Other Event {:?}", event)
            },
        }
    }
    
    let protocol = StreamProtocol::new("/ipfs/ping/1.0.0");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let start = Instant::now();

    for times_pinged in 0..times_to_ping {
        let stream = match control.open_stream(target_peer_id, protocol.clone()).await {
            Ok(stream) => stream,
            Err(error @ libp2p_stream::OpenStreamError::UnsupportedProtocol(_)) => {
                log::error!("UNSUPPORTED PROTOCOL {:?}", error);
                break;
            }
            Err(error) => {
                log::error!("{:?}", error);
                continue;
            }
        };
        let r = send_random_ping(stream).await;
        if let Err(e) = r {
            log::error!("STREAM ERROR ::: {:?}", e);
            continue;
        }
        log::info!("ping #{:?}", times_pinged+1);
    }

    let duration = start.elapsed();
    log::info!("Time taken to ping {} times: {:?}", times_to_ping, duration);
    log::info!("Average time taken to ping: {:?}", duration / times_to_ping as u32);

    Ok(())
}
