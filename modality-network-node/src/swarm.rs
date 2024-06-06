use anyhow::Result;

use libp2p::{identify, identity};
use libp2p::swarm;
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder};
use libp2p::ping;
use libp2p_noise;
use libp2p_stream;
use libp2p_yamux;
use libp2p::gossipsub;
use libp2p::request_response;
use std::time::Duration;

use crate::reqres;
use crate::gossip;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub stream: libp2p_stream::Behaviour,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub reqres: reqres::Behaviour,
    // pub gossip: gossip::Behaviour
    pub gossipsub: gossipsub::Behaviour,
    // kademlia: Kademlia<MemoryStore>,
    // relay: relay::Behaviour,
    // request_response: request_response::Behaviour<FileExchangeCodec>,
    // connection_limits: memory_connection_limits::Behaviour,
}

pub async fn create_swarm_with_behaviours(
    local_key: identity::Keypair,
    behaviour: Behaviour,
) -> Result<Swarm<Behaviour>> {
    let swarm = SwarmBuilder::with_existing_identity(local_key) // local_key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p_noise::Config::new,
            libp2p_yamux::Config::default,
        )?
        .with_websocket(libp2p_noise::Config::new, libp2p_yamux::Config::default)
        .await?
        .with_behaviour(|_key| behaviour)?
        .with_swarm_config(|cfg| {
            // Edit cfg here.
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        })
        .build();
    Ok(swarm)
}

pub async fn create_swarm(local_key: identity::Keypair) -> Result<Swarm<Behaviour>> {
    let identify_behaviour = identify::Behaviour::new(
        identify::Config::new("/ipfs/id/1.0.0".into(), local_key.public())
            .with_interval(std::time::Duration::from_secs(60)), // do this so we can get timeouts for dropped WebRTC connections
    );
    let ping_behaviour = ping::Behaviour::new(ping::Config::new());
    
    let reqres_behaviour = reqres::Behaviour::new(
        [(swarm::StreamProtocol::new(reqres::PROTOCOL), request_response::ProtocolSupport::Full)],
        request_response::Config::default()
    );

    let gossipsub_message_auth = gossipsub::MessageAuthenticity::Signed(local_key.clone());
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`. 
    let gossipsub_behaviour = gossipsub::Behaviour::new(gossipsub_message_auth, gossipsub_config).expect("Failed to create gossipsub behaviour");

    let behaviour = Behaviour {
        ping: ping_behaviour,
        identify: identify_behaviour,
        reqres: reqres_behaviour,
        stream: libp2p_stream::Behaviour::new(),
        gossipsub: gossipsub_behaviour,
    };
    let swarm = create_swarm_with_behaviours(local_key, behaviour).await?;
    Ok(swarm)
}