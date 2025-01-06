use crate::swarm;
use crate::reqres;
use crate::node::Node;

use anyhow::{Result};
use futures::prelude::*;
use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::SwarmEvent;
use libp2p::request_response;

pub async fn run(node: &mut Node, target: String, path: String, data: String) -> Result<()> {
    let ma = target.parse::<Multiaddr>().unwrap();

    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
    };

    node.swarm.dial(ma.clone())?;

    loop {
        match node.swarm.select_next_some().await {
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
        path: path.clone().to_string(),
        data: Some(serde_json::json!(data.clone())),
    };
    let target_request_id = node.swarm
        .behaviour_mut()
        .reqres
        .send_request(&target_peer_id, request);

    let _channel = loop {
        futures::select!(
            event = node.swarm.select_next_some() => match event {
              SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                request_response::Event::Message {
                  message: request_response::Message::Response { response, request_id, .. },
                  ..
                }
              )) => {
                if target_request_id == request_id {
                  log::debug!("response: {}", serde_json::to_string_pretty(&response).unwrap());
                  break;
                }
              }
              _ => {}
            }
        )
    };

    let _ = node.swarm.disconnect_peer_id(target_peer_id);

    loop {
        match node.swarm.select_next_some().await {
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                if peer_id == target_peer_id {
                    log::debug!("Connection closed with peer {:?}", peer_id);
                    break;
                }
            }
            _ => {}
        }
    }

    Ok(())
}
