
use anyhow::{Result};

// use libp2p::request_response::{self, ProtocolSupport};
use libp2p::{identify, identity};
use libp2p::PeerId;
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder};
// use libp2p::core::transport::dummy::DummyTransport;
// use libp2p::core::muxing::StreamMuxerBox;
use libp2p_noise;
use libp2p_yamux;
use libp2p_tls;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    identify: identify::Behaviour,
    relay: libp2p_relay::client::Behaviour,
    // gossipsub: gossipsub::Behaviour,
    // kademlia: Kademlia<MemoryStore>,
    // relay: relay::Behaviour,
    // request_response: request_response::Behaviour<FileExchangeCodec>,
    // connection_limits: memory_connection_limits::Behaviour,
}

pub async fn create_swarm(local_key: identity::Keypair) -> Result<Swarm<Behaviour>> {
    let local_peer_id = PeerId::from(local_key.public());
    log::debug!("Local peer id: {local_peer_id}");

    let identify_behaviour = identify::Behaviour::new(
        identify::Config::new("/ipfs/id/1.0.0".into(), local_key.public())
            .with_interval(std::time::Duration::from_secs(60)), // do this so we can get timeouts for dropped WebRTC connections
    );

    let swarm = SwarmBuilder::with_new_identity() // local_key)
      .with_tokio()
      .with_tcp(
          Default::default(),
          (libp2p_tls::Config::new, libp2p_noise::Config::new),
          libp2p_yamux::Config::default,
      )?
      // .with_quic()
      // .with_other_transport(|_key| DummyTransport::<(PeerId, StreamMuxerBox)>::new())?
      // .with_dns()?
      .with_websocket(
        (libp2p_tls::Config::new, libp2p_noise::Config::new),
        libp2p_yamux::Config::default,
      )
      .await?
      .with_relay_client(
          (libp2p_tls::Config::new, libp2p_noise::Config::new),
          libp2p_yamux::Config::default,
      )?
      .with_behaviour(|_key, relay| Behaviour {
        relay,
        identify: identify_behaviour,
      })?
      .with_swarm_config(|cfg| {
          // Edit cfg here.
          cfg
      })
      .build();

    Ok(swarm) 
}