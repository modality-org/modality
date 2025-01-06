use crate::node::Node;

use anyhow::{Result};
use futures::prelude::*;
use futures::future::{select, Either};

use std::time::Duration;

use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::SwarmEvent;
use libp2p::request_response;
use libp2p::kad;

pub async fn run(node: &mut Node) -> Result<()> {
  let tick_interval: Duration = Duration::from_secs(15);
  let mut tick = futures_timer::Delay::new(tick_interval);

  loop {
      match select(node.swarm.next(), &mut tick).await {
          Either::Left((event, _)) => match event.unwrap() {
              SwarmEvent::NewListenAddr { address, .. } => {
                  let address_with_p2p = address.clone().with(libp2p::multiaddr::Protocol::P2p(node.peerid));
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
                      // anyhow::bail!("Failed to dial peer");
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
                      node.swarm.behaviour_mut().reqres.send_response(channel, res).expect("failed to respond")
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
              SwarmEvent::Behaviour(crate::swarm::NodeBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, ..})) => {
                  match result {
                      kad::QueryResult::Bootstrap(result) => {
                          log::info!("Bootstrap result: {:?}", result);
                      }
                      kad::QueryResult::GetClosestPeers(result) => {
                          log::info!("GetClosestPeers result: {:?}", result);
                      }
                      kad::QueryResult::GetProviders(result) => {
                          log::info!("GetProviders result: {:?}", result);
                      }
                      kad::QueryResult::StartProviding(result) => {
                          log::info!("StartProviding result: {:?}", result);
                      }
                      kad::QueryResult::RepublishProvider(result) => {
                          log::info!("RepublishProvider result: {:?}", result);
                      }
                      kad::QueryResult::GetRecord(result) => {
                          log::info!("GetRecord result: {:?}", result);
                      }
                      kad::QueryResult::PutRecord(result) => {
                          log::info!("PutRecord result: {:?}", result);
                      }
                      kad::QueryResult::RepublishRecord(result) => {
                          log::info!("RepublishRecord result: {:?}", result);
                      }
                  }
              },
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
