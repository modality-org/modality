use crate::config::Config;
use anyhow::Result;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId};

use std::path::PathBuf;
use libp2p::multiaddr::Protocol;
use futures::future::{select, Either};
use libp2p::futures::StreamExt;
use std::time::Duration;
use libp2p::request_response;
use modality_utils::multiaddr_list::resolve_dns_multiaddrs;

pub struct Node {
    pub peerid: libp2p_identity::PeerId,
    pub node_keypair: libp2p_identity::Keypair,
    pub listeners: Vec<Multiaddr>,
    pub bootstrappers: Vec<Multiaddr>,
    pub swarm: crate::swarm::NodeSwarm
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
        let resolved_bootstrappers = resolve_dns_multiaddrs(config.bootstrappers.unwrap()).await?;
        let bootstrappers = exclude_multiaddresses_with_peerid(resolved_bootstrappers, peerid);
        let swarm = crate::swarm::create_swarm(node_keypair.clone()).await?;
        let node = Self { peerid, node_keypair, listeners, bootstrappers, swarm };
        Ok(node)
    }

    pub async fn setup(&mut self) -> Result<()> {
        // node.attach_storage(config.storage_path);
        for listener in self.listeners.clone() {
            self.swarm.listen_on(listener)?;
        }
        for bootstrapper in self.bootstrappers.clone() {
            log::info!("adding {bootstrapper:?}");
            self.swarm.dial(bootstrapper)?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let tick_interval: Duration = Duration::from_secs(15);
        let mut tick = futures_timer::Delay::new(tick_interval);

        loop {
            match select(self.swarm.next(), &mut tick).await {
                Either::Left((event, _)) => match event.unwrap() {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        let address_with_p2p = address.clone().with(libp2p::multiaddr::Protocol::P2p(self.peerid));
                        log::info!("Listening on {address_with_p2p:?}")
                    }
                    SwarmEvent::ConnectionEstablished { .. } => {
                        // if peer_id == target_peer_id {
                        //     log::debug!("Connected to peer {:?}", peer_id);
                        //     // do we ever need to wait for correct transport upgrade event?
                        //     // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        //     break;
                        // }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        if let Some(peer_id) = peer_id {
                            log::error!("Failed to dial peer {:?}", peer_id);
                            log::error!("Error: {:?}", error);
                            anyhow::bail!("Failed to dial peer");
                        }
                    }
                    SwarmEvent::Behaviour(crate::swarm::NodeBehaviourEvent::Reqres(
                        request_response::Event::Message { message, .. },
                    )) => match message {
                        request_response::Message::Request {
                            request,
                            channel,
                            .. // request, channel, ..
                        } => {
                            log::info!("reqres request");
                            let res = crate::reqres::handle_request(request).await?;
                            self.swarm.behaviour_mut().reqres.send_response(channel, res).expect("failed to respond")
                        }
                        request_response::Message::Response {
                            ..
                            // request_id,
                            // response,
                        } => {
                            log::info!("reqres response")
                        }
                    },
                    // SwarmEvent::Behaviour(event) => {
                    //     log::info!("SwarmEvent::Behaviour event {:?}", event);
                    //     match event {
                    //         swarm::BehaviourEvent::Identify(_) => {
                    //             log::info!("Identify Behaviour event");
                    //         }
                    //         swarm::BehaviourEvent::Ping(_) => {
                    //             log::info!("Ping Behaviour event");
                    //         }
                    //         swarm::BehaviourEvent::Stream(_) => {
                    //             log::info!("Stream Behaviour event");
                    //         }
                    //         swarm::BehaviourEvent::Reqres(_) => {
                    //             log::info!("Reqres Behaviour event");
                    //         }
                    //         // _ => {
                    //         //     log::info!("Other Swarm Behaviour event {:?}", event);
                    //         // }
                    //     }
                    // }
                    event => {
                        log::info!("Other Node Event {:?}", event)
                    },
                },
                Either::Right(_) => {
                    log::debug!("tick");
                    tick = futures_timer::Delay::new(tick_interval);
                }
            }
        }
    }
}

pub fn exclude_multiaddresses_with_peerid(ma: Vec<Multiaddr>, peerid: PeerId) -> Vec<Multiaddr> {
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