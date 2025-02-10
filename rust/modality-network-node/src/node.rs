use anyhow::Result;
use futures::prelude::*;
use libp2p::gossipsub::IdentTopic;
use libp2p::request_response::OutboundRequestId;
use std::path::PathBuf;
use std::time::Duration;

use libp2p::multiaddr::Protocol;
use libp2p::request_response;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId};

use crate::config::Config;
use crate::consensus::net_comm::NetComm;
use crate::reqres;
use crate::swarm;
use modality_network_datastore::NetworkDatastore;
use modality_utils::multiaddr_list::resolve_dns_multiaddrs;

pub struct Node {
    pub peerid: libp2p_identity::PeerId,
    pub node_keypair: libp2p_identity::Keypair,
    pub listeners: Vec<Multiaddr>,
    pub bootstrappers: Vec<Multiaddr>,
    pub swarm: crate::swarm::NodeSwarm,
    pub datastore: NetworkDatastore,
}

impl Node {
    pub async fn from_config_filepath(config_filepath: PathBuf) -> Result<Node> {
        let config = Config::from_filepath(&config_filepath)?;
        Node::from_config(config).await
    }

    pub async fn from_config(config: Config) -> Result<Node> {
        let node_keypair = config.get_libp2p_keypair().await?;
        let peerid = node_keypair.public().to_peer_id();
        let listeners = config.listeners.unwrap_or_default();
        let resolved_bootstrappers =
            resolve_dns_multiaddrs(config.bootstrappers.unwrap_or_default()).await?;
        let bootstrappers = exclude_multiaddresses_with_peerid(resolved_bootstrappers, peerid);
        let swarm = crate::swarm::create_swarm(node_keypair.clone()).await?;
        let datastore = if let Some(storage_path) = config.storage_path {
            NetworkDatastore::create_in_directory(&storage_path)?
        } else {
            NetworkDatastore::create_in_memory()?
        };
        let node = Self {
            peerid,
            node_keypair,
            listeners,
            bootstrappers,
            swarm,
            datastore,
        };
        Ok(node)
    }

    pub async fn setup(&mut self) -> Result<()> {
        // node.attach_storage(config.storage_path);
        for listener in self.listeners.clone() {
            self.swarm.listen_on(listener.clone())?;
            self.swarm.add_external_address(listener.clone());
        }
        for bootstrapper in self.bootstrappers.clone() {
            if let Some(peer_id) = extract_peer_id(bootstrapper.clone()) {
                log::info!("Adding Bootstrap Peer: {peer_id:?} {bootstrapper:?}");
                self.swarm
                    .add_peer_address(peer_id.clone(), bootstrapper.clone());
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id.clone(), bootstrapper.clone());
            } else {
                log::info!("skipping bootstrapper missing peerid: {bootstrapper:?}");
            }
        }
        Ok(())
    }

    pub async fn get_consensus_communication(self) -> NetComm {
        NetComm::new(self)
    }

    pub async fn send_request_only(
        &mut self,
        target_peer_id: PeerId,
        path: String,
        data: String,
    ) -> Result<OutboundRequestId> {
        let request = reqres::Request {
            path: path.clone().to_string(),
            data: Some(serde_json::json!(data.clone())),
        };
        let req_id = self
            .swarm
            .behaviour_mut()
            .reqres
            .send_request(&target_peer_id, request);
        Ok(req_id)
    }

    pub async fn send_request(
        &mut self,
        target_peer_id: PeerId,
        path: String,
        data: String,
    ) -> Result<reqres::Response> {
        let target_request_id = self.send_request_only(target_peer_id, path, data).await?;
        let res: reqres::Response;
        loop {
            futures::select!(
                event = self.swarm.select_next_some() => match event {
                  SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                    request_response::Event::Message {
                      message: request_response::Message::Response { response, request_id, .. },
                      ..
                    }
                  )) => {
                    if target_request_id == request_id {
                        // log::debug!("response: {}", serde_json::to_string_pretty(&response).unwrap());
                        res = response.clone();
                        break;
                    }
                  }
                  _ => {}
                }
            )
        }
        Ok(res)
    }

    pub async fn connect_to_peer_multiaddr(&mut self, ma: Multiaddr) -> Result<()> {
        self.swarm.dial(ma.clone())?;

        let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
            anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
        };

        loop {
            match self.swarm.select_next_some().await {
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

        Ok(())
    }

    pub async fn disconnect_from_peer_id(&mut self, target_peer_id: PeerId) -> Result<()> {
        let _ = self.swarm.disconnect_peer_id(target_peer_id);

        loop {
            match self.swarm.select_next_some().await {
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

    pub async fn publish_gossip(&mut self, topic: String, data: String) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(topic), data)?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let ids: Vec<_> = self.swarm.connected_peers().cloned().collect();
        for peer_id in ids {
            self.swarm
                .disconnect_peer_id(peer_id)
                .map_err(|_| anyhow::anyhow!("Failed to disconnect from peer {}", peer_id))?;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}

fn extract_peer_id(multiaddr: Multiaddr) -> Option<PeerId> {
    let protocols: Vec<libp2p::multiaddr::Protocol> = multiaddr.iter().collect();
    let last_protocol = protocols.last()?;

    // Check if it's a p2p protocol and extract the peer ID
    match last_protocol {
        Protocol::P2p(peer_id) => Some(peer_id.clone()),
        _ => None,
    }
}

fn exclude_multiaddresses_with_peerid(ma: Vec<Multiaddr>, peerid: PeerId) -> Vec<Multiaddr> {
    ma.into_iter()
        .filter(|addr| {
            if let Some(Protocol::P2p(addr_peerid)) = addr.iter().last() {
                addr_peerid != peerid
            } else {
                true
            }
        })
        .collect()
}
