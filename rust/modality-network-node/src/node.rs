use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

use libp2p::{Multiaddr, PeerId};
use libp2p::multiaddr::Protocol;

use modality_utils::multiaddr_list::resolve_dns_multiaddrs;
use modality_network_datastore::NetworkDatastore;
use crate::consensus::net_comm::NetComm;
use crate::config::Config;

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
        let resolved_bootstrappers = resolve_dns_multiaddrs(config.bootstrappers.unwrap_or_default()).await?;
        let bootstrappers = exclude_multiaddresses_with_peerid(resolved_bootstrappers, peerid);
        let swarm = crate::swarm::create_swarm(node_keypair.clone()).await?;
        let datastore = if let Some(storage_path) = config.storage_path {
            NetworkDatastore::create_in_directory(&storage_path)?
        } else {
            NetworkDatastore::create_in_memory()?
        };
        let node = Self { peerid, node_keypair, listeners, bootstrappers, swarm, datastore };
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
                self.swarm.add_peer_address(peer_id.clone(), bootstrapper.clone());
                self.swarm.behaviour_mut().kademlia.add_address(&peer_id.clone(), bootstrapper.clone());
            } else {
                log::info!("skipping bootstrapper missing peerid: {bootstrapper:?}");
            }
        }
        Ok(())
    }

    pub async fn get_consensus_communication(node: &'static mut Self) -> NetComm {
        NetComm::new(node)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let ids: Vec<_> = self.swarm
            .connected_peers()
            .cloned()
            .collect();
        for peer_id in ids {
            self.swarm.disconnect_peer_id(peer_id)
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