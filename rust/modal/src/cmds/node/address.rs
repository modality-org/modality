use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use libp2p::Multiaddr;

use modal_node::config_resolution::load_config_with_node_dir;

#[derive(Debug, Parser)]
#[command(about = "Display the listening addresses of a node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Show only one address
    #[clap(long, short = '1')]
    pub one: bool,

    /// Prefer public IP addresses
    #[clap(long)]
    pub prefer_public: bool,

    /// Prefer local/loopback IP addresses
    #[clap(long)]
    pub prefer_local: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Get the peer ID
    let peer_id = config.id.as_ref()
        .context("No peer ID found in config")?;
    
    // Get the listeners
    let listeners = config.listeners.as_ref()
        .context("No listeners configured")?;
    
    if listeners.is_empty() {
        anyhow::bail!("Node has no listening addresses configured");
    }
    
    // Parse peer ID once
    let peer_id_parsed: libp2p::PeerId = peer_id.parse()
        .context("Failed to parse peer ID")?;
    
    // Build addresses with /p2p/<peer_id> appended
    let mut addresses: Vec<Multiaddr> = Vec::new();
    
    for listener in listeners {
        // Clone the listener multiaddr and append peer ID
        let mut addr = listener.clone();
        addr.push(libp2p::multiaddr::Protocol::P2p(peer_id_parsed));
        addresses.push(addr);
    }
    
    // Sort addresses based on preferences
    if opts.prefer_public {
        addresses.sort_by(|a, b| {
            let a_public = is_public_address(a);
            let b_public = is_public_address(b);
            b_public.cmp(&a_public) // Public addresses first
        });
    } else if opts.prefer_local {
        addresses.sort_by(|a, b| {
            let a_local = is_local_address(a);
            let b_local = is_local_address(b);
            b_local.cmp(&a_local) // Local addresses first
        });
    }
    
    // Output addresses
    if opts.one {
        if let Some(addr) = addresses.first() {
            println!("{}", addr);
        }
    } else {
        for addr in addresses {
            println!("{}", addr);
        }
    }
    
    Ok(())
}

/// Check if a multiaddr represents a public IP address
fn is_public_address(addr: &Multiaddr) -> bool {
    use libp2p::multiaddr::Protocol;
    
    for protocol in addr.iter() {
        match protocol {
            Protocol::Ip4(ip) => {
                return !ip.is_loopback() 
                    && !ip.is_private() 
                    && !ip.is_link_local()
                    && !ip.is_unspecified();
            }
            Protocol::Ip6(ip) => {
                return !ip.is_loopback() 
                    && !ip.is_unspecified();
            }
            _ => continue,
        }
    }
    false
}

/// Check if a multiaddr represents a local/loopback address
fn is_local_address(addr: &Multiaddr) -> bool {
    use libp2p::multiaddr::Protocol;
    
    for protocol in addr.iter() {
        match protocol {
            Protocol::Ip4(ip) => {
                return ip.is_loopback() || ip == std::net::Ipv4Addr::new(127, 0, 0, 1);
            }
            Protocol::Ip6(ip) => {
                return ip.is_loopback();
            }
            _ => continue,
        }
    }
    false
}

