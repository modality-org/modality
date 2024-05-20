use anyhow::Result;

use libp2p::{identify, identity};
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder};
use libp2p::ping;
use libp2p_noise;
use libp2p_stream;
use libp2p_yamux;
use std::time::Duration;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub stream: libp2p_stream::Behaviour,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    // gossipsub: gossipsub::Behaviour,
    // kademlia: Kademlia<MemoryStore>,
    // relay: relay::Behaviour,
    // request_response: request_response::Behaviour<FileExchangeCodec>,
    // connection_limits: memory_connection_limits::Behaviour,
}

#[derive(NetworkBehaviour)]
pub struct StreamBehaviour {
    pub stream: libp2p_stream::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
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
    let behaviour = Behaviour {
        ping: ping_behaviour,
        identify: identify_behaviour,
        stream: libp2p_stream::Behaviour::new(),
    };
    let swarm = create_swarm_with_behaviours(local_key, behaviour).await?;
    Ok(swarm)
}