use anyhow::Result;

use libp2p::ping;
use libp2p::request_response;
use libp2p::swarm;
use libp2p::{identify, identity};
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder};
use std::time::Duration;

use crate::reqres;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    // pub stream: libp2p_stream::Behaviour,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub reqres: reqres::Behaviour,
    // gossipsub: gossipsub::Behaviour,
}

pub type NodeSwarm = Swarm<NodeBehaviour>;

pub async fn create_swarm(local_key: identity::Keypair) -> Result<NodeSwarm> {
    // let stream_behaviour = libp2p_stream::Behaviour::new();

    let identify_behaviour = identify::Behaviour::new(
        identify::Config::new("/ipfs/id/1.0.0".into(), local_key.public())
            .with_interval(std::time::Duration::from_secs(60)), // do this so we can get timeouts for dropped WebRTC connections
    );
    let ping_behaviour = ping::Behaviour::new(ping::Config::new());

    let reqres_behaviour = reqres::Behaviour::new(
        [(swarm::StreamProtocol::new(reqres::PROTOCOL), request_response::ProtocolSupport::Full)],
        request_response::Config::default()
    );

    let behaviour = NodeBehaviour {
        // stream: stream_behaviour,
        ping: ping_behaviour,
        identify: identify_behaviour,
        reqres: reqres_behaviour,
    };
    let swarm = create_swarm_with_behaviours(local_key, behaviour).await?;
    Ok(swarm)
}

pub async fn create_swarm_with_behaviours(
    local_key: identity::Keypair,
    behaviour: NodeBehaviour,
) -> Result<NodeSwarm> {
    let swarm0 = SwarmBuilder::with_existing_identity(local_key);
    let swarm1 = swarm0.with_tokio();
    let swarm2 = swarm1.with_tcp(
        libp2p::tcp::Config::default(),
        libp2p::noise::Config::new,
        libp2p::yamux::Config::default,
    )?;
    let swarm3 = swarm2
        .with_websocket(libp2p::noise::Config::new, libp2p::yamux::Config::default)
        .await?;
    let swarm4 = swarm3
        .with_behaviour(|_key| behaviour)?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        });
    let swarm = swarm4.build();
    Ok(swarm)
}
