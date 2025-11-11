use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use modal_node::config::Config;
use modal_node::inspection::{InspectionData, InspectionLevel, NodeStatus};
use modal_datastore::NetworkDatastore;

#[derive(Debug, Parser)]
#[command(about = "Inspect a Modality node's state")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: PathBuf,

    /// Target node to inspect (multiaddr format). If not specified, inspects local node.
    #[clap(long)]
    pub target: Option<String>,

    /// Inspection detail level: basic, full, network, datastore, mining
    #[clap(long, default_value = "basic")]
    pub level: String,

    /// Output raw JSON instead of pretty-printed format
    #[clap(long)]
    pub json: bool,

    /// Force direct datastore query (skip reqres even if node is running)
    #[clap(long)]
    pub offline: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Parse inspection level
    let level: InspectionLevel = opts.level.parse()
        .map_err(|e: String| anyhow::anyhow!("{}", e))?;

    // Load node config
    let config = Config::from_filepath(&opts.config)
        .context("Failed to load node configuration")?;

    let mut inspection_data: Option<InspectionData> = None;

    // Try reqres first if not in offline mode
    if !opts.offline && opts.target.is_some() {
        match try_reqres_inspect(&config, opts.target.as_ref().unwrap(), level).await {
            Ok(data) => {
                inspection_data = Some(data);
            }
            Err(e) => {
                log::debug!("Reqres inspection failed: {}, falling back to offline mode", e);
            }
        }
    }

    // If reqres didn't work or offline mode, query datastore directly
    if inspection_data.is_none() {
        inspection_data = Some(query_datastore_directly(&config, level).await?);
    }

    let data = inspection_data.unwrap();

    // Output results
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print_pretty(&data);
    }

    Ok(())
}

/// Try to inspect via reqres
async fn try_reqres_inspect(
    config: &Config,
    target: &str,
    level: InspectionLevel,
) -> Result<InspectionData> {
    use libp2p::multiaddr::Multiaddr;
    use modal_node::node::Node;

    // Create a temporary node for making the request
    let mut node = Node::from_config(config.clone()).await?;

    let ma = target.parse::<Multiaddr>()
        .context("Invalid multiaddr format")?;

    let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
        anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
    };

    node.connect_to_peer_multiaddr(ma.clone()).await?;

    let request_data = serde_json::json!({
        "level": level.to_string()
    });

    let res = node.send_request(
        target_peer_id,
        "/inspect".to_string(),
        serde_json::to_string(&request_data)?,
    ).await?;

    node.disconnect_from_peer_id(target_peer_id).await?;

    if !res.ok {
        anyhow::bail!("Inspection request failed: {:?}", res.errors);
    }

    let data: InspectionData = serde_json::from_value(res.data.unwrap_or_default())
        .context("Failed to parse inspection response")?;

    Ok(data)
}

/// Query datastore directly (offline mode)
async fn query_datastore_directly(
    config: &Config,
    level: InspectionLevel,
) -> Result<InspectionData> {
    let storage_path = config.storage_path.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No storage_path configured in node config"))?;

    let mut datastore = NetworkDatastore::create_in_directory(storage_path)
        .context("Failed to open datastore")?;

    let inspection_data = modal_node::reqres::inspect::get_datastore_inspection(&mut datastore, level).await?;

    // Get peer ID from config if available
    let peer_id = if let Ok(keypair) = config.get_libp2p_keypair().await {
        keypair.public().to_peer_id().to_string()
    } else {
        "unknown".to_string()
    };

    // Update the inspection data with correct peer ID and offline status
    let mut data = inspection_data;
    data.peer_id = peer_id;
    data.status = NodeStatus::Offline;

    Ok(data)
}

/// Pretty-print inspection data
fn print_pretty(data: &InspectionData) {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║              Modality Node Inspection Report                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Basic info
    println!("📋 Basic Information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Peer ID: {}", data.peer_id);
    println!("  Status: {}", match data.status {
        NodeStatus::Running => "🟢 Running",
        NodeStatus::Offline => "🔴 Offline",
    });
    println!();

    // Datastore info
    if let Some(ref ds) = data.datastore {
        println!("💾 Datastore");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Total Blocks: {}", ds.total_blocks);
        if let Some((first, last)) = ds.block_range {
            println!("  Block Range: {} → {}", first, last);
        }
        if let Some(height) = ds.chain_tip_height {
            println!("  Chain Tip Height: {}", height);
        }
        if let Some(ref hash) = ds.chain_tip_hash {
            println!("  Chain Tip Hash: {}...{}", &hash[0..8.min(hash.len())], &hash[hash.len().saturating_sub(8)..]);
        }
        if let Some(epochs) = ds.epochs {
            println!("  Epochs: {}", epochs);
        }
        if let Some(miners) = ds.unique_miners {
            println!("  Unique Miners: {}", miners);
        }
        println!();
    }

    // Network info
    if let Some(ref net) = data.network {
        println!("🌐 Network");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Connected Peers: {}", net.connected_peers);
        if let Some(ref peer_list) = net.connected_peer_list {
            println!("  Peer List:");
            for peer in peer_list {
                println!("    - {}", peer);
            }
        }
        println!("  Listeners: {}", net.listeners.len());
        for listener in &net.listeners {
            println!("    - {}", listener);
        }
        println!("  Bootstrappers: {}", net.bootstrappers.len());
        for bootstrapper in &net.bootstrappers {
            println!("    - {}", bootstrapper);
        }
        println!();
    }

    // Mining info
    if let Some(ref mining) = data.mining {
        println!("⛏️  Mining");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Is Mining: {}", if mining.is_mining { "Yes" } else { "No" });
        if let Some(ref nominees) = mining.nominees {
            println!("  Nominees: {}", nominees.len());
            for nominee in nominees {
                println!("    - {}", nominee);
            }
        }
        if let Some(hashrate) = mining.current_hashrate {
            println!("  Current Hashrate: {:.2} H/s", hashrate);
        }
        if let Some(total) = mining.total_hashes {
            println!("  Total Hashes: {}", total);
        }
        println!();
    }

    println!("✅ Inspection complete!");
}

