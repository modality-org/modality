use anyhow::Result;

use libp2p::ping;
use libp2p::request_response;
use libp2p::swarm;
use libp2p::{identify, identity};
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder};
use libp2p::kad;
use libp2p::gossipsub;
use std::time::Duration;

use crate::reqres;
// use crate::gossip;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    // pub stream: libp2p_stream::Behaviour,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub reqres: reqres::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

pub type NodeSwarm = Swarm<NodeBehaviour>;

pub async fn create_swarm(local_key: identity::Keypair) -> Result<NodeSwarm> {
    create_swarm_with_metadata(local_key, None, None).await
}

pub async fn create_swarm_with_status_url(local_key: identity::Keypair, status_url: Option<String>) -> Result<NodeSwarm> {
    create_swarm_with_metadata(local_key, status_url, None).await
}

pub async fn create_swarm_with_metadata(local_key: identity::Keypair, status_url: Option<String>, role: Option<String>) -> Result<NodeSwarm> {
    // let stream_behaviour = libp2p_stream::Behaviour::new();

    // Create agent version string that includes status_url and role if provided
    // Format: "modal-node/0.1.0;status_url=https://...;role=Miner"
    let mut agent_parts = vec!["modal-node/0.1.0".to_string()];
    if let Some(url) = status_url {
        agent_parts.push(format!("status_url={}", url));
    }
    if let Some(r) = role {
        agent_parts.push(format!("role={}", r));
    }
    let agent_version = agent_parts.join(";");

    let identify_behaviour = identify::Behaviour::new(
        identify::Config::new("/ipfs/id/1.0.0".into(), local_key.public())
            .with_interval(std::time::Duration::from_secs(60)) // do this so we can get timeouts for dropped WebRTC connections
            .with_agent_version(agent_version)
    );
    let ping_behaviour = ping::Behaviour::new(ping::Config::new());

    let reqres_behaviour = reqres::Behaviour::new(
        [(swarm::StreamProtocol::new(reqres::PROTOCOL), request_response::ProtocolSupport::Full)],
        request_response::Config::default()
            .with_request_timeout(Duration::from_secs(60)) // Longer timeout for large transfers
    );


    let peer_id = local_key.clone().public().to_peer_id();
    let kademlia_behaviour = kad::Behaviour::new(
        peer_id,
        kad::store::MemoryStore::new(peer_id),
    );

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .allow_self_origin(true)
        .build()?;
    let gossipsub_behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config
    ).unwrap();

    let behaviour = NodeBehaviour {
        // stream: stream_behaviour,
        ping: ping_behaviour,
        identify: identify_behaviour,
        reqres: reqres_behaviour,
        gossipsub: gossipsub_behaviour,
        kademlia: kademlia_behaviour,
    };
    let swarm = create_swarm_with_behaviours(local_key, behaviour).await?;

    Ok(swarm)
}

pub async fn create_swarm_with_behaviours(
    local_key: identity::Keypair,
    behaviour: NodeBehaviour,
) -> Result<NodeSwarm> {
    let swarm = SwarmBuilder::with_existing_identity(local_key);
    let swarm = swarm.with_tokio();
    let swarm = swarm.with_tcp(
        libp2p::tcp::Config::default(),
        libp2p::noise::Config::new,
        libp2p::yamux::Config::default,
    )?;
    let swarm = swarm.with_dns()?;
    let swarm = swarm
        .with_websocket(libp2p::noise::Config::new, libp2p::yamux::Config::default)
        .await?;
    let swarm = swarm
        .with_behaviour(|_key| behaviour)?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        });
    let swarm = swarm.build();
    Ok(swarm)
}
