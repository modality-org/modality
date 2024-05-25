use crate::config_file;
use crate::identity_utils;
use crate::swarm;

use crate::reqres;
use anyhow::{Context, Result};
use clap::Parser;
use futures::prelude::*;
use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::SwarmEvent;
use std::borrow::Borrow;
use libp2p::request_response;

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

    #[clap(long)]
    path: String,

    #[clap(long, default_value = "{}")]
    data: String
}

pub async fn run(opts: &Opts) -> Result<()> {
    let config =
        config_file::read_or_create_config(&opts.config).context("Failed to read config")?;
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
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if peer_id == target_peer_id {
                    log::debug!("Connected to peer {:?}", peer_id);
                    // do we ever need to wait for correct transport upgrade event?
                    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    break;
                }
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    log::error!("Failed to dial peer {:?}", peer_id);
                    log::error!("Error: {:?}", error);
                    anyhow::bail!("Failed to dial peer");
                }
            }
            event => {
                log::debug!("Other Event {:?}", event)
            }
        }
    }

    let request = reqres::Request {
        path: opts.path.clone().to_string(),
        data: serde_json::json!(opts.data.clone()),
    };
    let target_request_id = swarm
        .behaviour_mut()
        .reqres
        .send_request(&target_peer_id, request);

    let _channel = loop {
        futures::select!(
            event = swarm.select_next_some() => match event {
              SwarmEvent::Behaviour(swarm::BehaviourEvent::Reqres(
                request_response::Event::Message {
                  message: request_response::Message::Response { response, request_id, .. },
                  ..
                }
              )) => {
                if target_request_id == request_id {
                  println!("{}", serde_json::to_string_pretty(&response.data).unwrap());
                  break;
                }
              }
              _ => {}
            }
        )
    };

    Ok(())
}
