use anyhow::Result;

use libp2p::kad::store::MemoryStore;
use libp2p::kad::BootstrapResult;
use libp2p::multiaddr::Protocol;
use libp2p::{identify, identity, Multiaddr};
use libp2p::swarm;
use libp2p::{swarm::NetworkBehaviour, swarm::Swarm, SwarmBuilder, kad};
use libp2p::ping;

use libp2p_noise;
use libp2p_stream;
use libp2p_yamux;
use libp2p::gossipsub;
use libp2p::request_response;
use std::time::Duration;
use std::num::NonZeroUsize;


use crate::reqres;


#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub stream: libp2p_stream::Behaviour,
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub reqres: reqres::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

pub async fn create_swarm_with_behaviours(
    local_key: identity::Keypair,
    behaviour: Behaviour,
) -> Result<Swarm<Behaviour>> {
    let local_peer_id = local_key.public().to_peer_id();

     // Configure Kademlia
     let mut kademlia_config = kad::Config::default();
     kademlia_config.set_protocol_names(["/myapp/kad/1.0.0"]);
     kademlia_config.set_query_timeout(Duration::from_secs(5 * 60));
     
     let store = MemoryStore::new(local_peer_id);
     let mut kademlia = Kademlia::with_config(local_peer_id, store, kademlia_config);
 

    let bootstrap_peers = vec![
        "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN"
    ];


    for peer in bootstrap_peers {
        if let Ok(addr) = peer.parse::<Multiaddr>() {
            if let Some(Protocol::P2p(peer_id)) = addr.iter().last() {
                behaviour.kademlia.add_address(&peer_id, addr.clone());
            }
        }
    }

    behaviour.kademlia = kademlia;


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
    swarm.behaviour_mut().kademlia.bootstrap()?;

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
        kademlia: kad::Behaviour::new(kad::store::MemoryStore::new(local_key.public().into()), kad::Config::default()),
    };
    let swarm = create_swarm_with_behaviours(local_key, behaviour).await?;
    Ok(swarm)
}