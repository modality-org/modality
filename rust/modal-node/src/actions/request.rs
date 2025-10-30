use crate::reqres;
use crate::node::Node;

use anyhow::{Result};
use libp2p::multiaddr::Multiaddr;

pub async fn run(node: &mut Node, target: String, path: String, data: String) -> Result<reqres::Response> {
    let ma = target.parse::<Multiaddr>().unwrap();
    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
    };

    node.connect_to_peer_multiaddr(ma.clone()).await?;

    let res = node.send_request(target_peer_id, path.clone(), data.clone()).await?;
    if res.ok {
        log::debug!("response: {}", serde_json::to_string_pretty(&res).unwrap());
    }
    
    node.disconnect_from_peer_id(target_peer_id).await?;

    Ok(res)
}
