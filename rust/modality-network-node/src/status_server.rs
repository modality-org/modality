use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

use modality_network_datastore::NetworkDatastore;
use modality_network_datastore::models::MinerBlock;

/// Start HTTP status server on the specified port
pub async fn start_status_server(
    port: u16,
    peerid: libp2p_identity::PeerId,
    datastore: Arc<Mutex<NetworkDatastore>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    let status_route = warp::path::end()
        .and(warp::get())
        .and(with_peerid(peerid))
        .and(with_datastore(datastore.clone()))
        .and(with_swarm(swarm.clone()))
        .and(with_listeners(listeners.clone()))
        .and_then(status_handler);

    let routes = status_route;

    log::info!("Starting HTTP status server on http://0.0.0.0:{}", port);

    let server = warp::serve(routes).bind(([0, 0, 0, 0], port));

    let handle = tokio::spawn(async move {
        server.await;
    });

    Ok(handle)
}

fn with_peerid(
    peerid: libp2p_identity::PeerId,
) -> impl Filter<Extract = (libp2p_identity::PeerId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peerid)
}

fn with_datastore(
    datastore: Arc<Mutex<NetworkDatastore>>,
) -> impl Filter<Extract = (Arc<Mutex<NetworkDatastore>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || datastore.clone())
}

fn with_swarm(
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
) -> impl Filter<Extract = (Arc<Mutex<crate::swarm::NodeSwarm>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || swarm.clone())
}

fn with_listeners(
    listeners: Vec<libp2p::Multiaddr>,
) -> impl Filter<Extract = (Vec<libp2p::Multiaddr>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || listeners.clone())
}

async fn status_handler(
    peerid: libp2p_identity::PeerId,
    datastore: Arc<Mutex<NetworkDatastore>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    listeners: Vec<libp2p::Multiaddr>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Get connected peers information
    let peer_info = {
        let swarm_lock = swarm.lock().await;
        swarm_lock.connected_peers().cloned().collect::<Vec<_>>()
    };
    let connected_peers = peer_info.len();
    
    // Get node status information
    let ds = datastore.lock().await;
    let current_round = ds.get_current_round().await.unwrap_or(0);
    let latest_round = ds.find_max_int_key("/blocks/round").await.ok().flatten().unwrap_or(0);
    
    // Get miner blocks information
    let miner_blocks = MinerBlock::find_all_canonical(&ds).await.unwrap_or_default();
    let total_miner_blocks = miner_blocks.len();
    
    // Get latest block for current difficulty
    let latest_block = miner_blocks.iter().max_by_key(|b| b.index);
    let current_difficulty = latest_block
        .map(|b| b.difficulty.clone())
        .unwrap_or_else(|| "0".to_string());
    let current_epoch = latest_block.map(|b| b.epoch).unwrap_or(0);
    
    // Calculate cumulative difficulty
    let cumulative_difficulty: u128 = miner_blocks
        .iter()
        .filter_map(|block| block.difficulty.parse::<u128>().ok())
        .sum();
    
    // Count blocks mined by this node
    let peerid_str = peerid.to_string();
    let blocks_mined_by_node = miner_blocks
        .iter()
        .filter(|block| block.nominated_peer_id == peerid_str)
        .count();
    
    // Get Block 0 (genesis block)
    let block_0 = MinerBlock::find_canonical_by_index(&ds, 0).await.ok().flatten();
    
    // Get last 80 blocks (sorted by index descending)
    let mut recent_blocks = miner_blocks.clone();
    recent_blocks.sort_by(|a, b| b.index.cmp(&a.index));
    recent_blocks.truncate(80);
    
    // Get first 40 blocks (sorted by index ascending)
    let mut first_blocks = miner_blocks.clone();
    first_blocks.sort_by(|a, b| a.index.cmp(&b.index));
    first_blocks.truncate(40);
    
    drop(ds);

    // Build blocks table HTML for recent blocks (last 80)
    let blocks_html = if recent_blocks.is_empty() {
        "<tr><td colspan='5' style='text-align: center; padding: 20px; color: #666;'>No blocks yet</td></tr>".to_string()
    } else {
        recent_blocks
            .iter()
            .map(|block| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                    block.index,
                    block.epoch,
                    if block.hash.len() > 16 {
                        format!("{}...{}", &block.hash[..8], &block.hash[block.hash.len()-8..])
                    } else {
                        block.hash.clone()
                    },
                    if block.nominated_peer_id.len() > 20 {
                        format!("{}...{}", &block.nominated_peer_id[..10], &block.nominated_peer_id[block.nominated_peer_id.len()-10..])
                    } else {
                        block.nominated_peer_id.clone()
                    },
                    block.timestamp
                )
            })
            .collect::<Vec<_>>()
            .join("\n                    ")
    };

    // Build blocks table HTML for first 40 blocks
    let first_blocks_html = if first_blocks.is_empty() {
        "<tr><td colspan='5' style='text-align: center; padding: 20px; color: #666;'>No blocks yet</td></tr>".to_string()
    } else {
        first_blocks
            .iter()
            .map(|block| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                    block.index,
                    block.epoch,
                    if block.hash.len() > 16 {
                        format!("{}...{}", &block.hash[..8], &block.hash[block.hash.len()-8..])
                    } else {
                        block.hash.clone()
                    },
                    if block.nominated_peer_id.len() > 20 {
                        format!("{}...{}", &block.nominated_peer_id[..10], &block.nominated_peer_id[block.nominated_peer_id.len()-10..])
                    } else {
                        block.nominated_peer_id.clone()
                    },
                    block.timestamp
                )
            })
            .collect::<Vec<_>>()
            .join("\n                    ")
    };

    // Build Block 0 HTML
    let block_0_html = if let Some(block) = &block_0 {
        format!(
            r#"<div class="status-item">
            <span class="label">Index:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Hash:</span>
            <span class="value"><code>{}</code></span>
        </div>
        <div class="status-item">
            <span class="label">Epoch:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Timestamp:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Previous Hash:</span>
            <span class="value"><code>{}</code></span>
        </div>
        <div class="status-item">
            <span class="label">Data Hash:</span>
            <span class="value"><code>{}</code></span>
        </div>
        <div class="status-item">
            <span class="label">Difficulty:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Nominated Peer:</span>
            <span class="value"><code>{}</code></span>
        </div>"#,
            block.index,
            block.hash,
            block.epoch,
            block.timestamp,
            block.previous_hash,
            block.data_hash,
            block.difficulty,
            block.nominated_peer_id
        )
    } else {
        r#"<div class="status-item">
            <span class="label" style="color: #666;">Block 0 not found</span>
        </div>"#.to_string()
    };

    // Build peers list HTML
    let peers_html = if peer_info.is_empty() {
        "<tr><td style='text-align: center; padding: 20px; color: #666;'>No connected peers</td></tr>".to_string()
    } else {
        peer_info
            .iter()
            .map(|peer_id| {
                format!(
                    "<tr><td><code>{}</code></td></tr>",
                    peer_id
                )
            })
            .collect::<Vec<_>>()
            .join("\n                    ")
    };

    // Build HTML response
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Modality Node Status</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            max-width: 1200px;
            margin: 40px auto;
            padding: 20px;
            background: #0f0f0f;
            color: #e0e0e0;
        }}
        h1 {{
            color: #4a9eff;
            border-bottom: 2px solid #4a9eff;
            padding-bottom: 10px;
        }}
        h2 {{
            color: #4a9eff;
            font-size: 1.3em;
            margin-top: 0;
        }}
        .status-card {{
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
        }}
        .status-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin: 20px 0;
        }}
        .status-item {{
            display: flex;
            justify-content: space-between;
            padding: 12px 0;
            border-bottom: 1px solid #2a2a2a;
        }}
        .status-item:last-child {{
            border-bottom: none;
        }}
        .label {{
            font-weight: 600;
            color: #888;
        }}
        .value {{
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
            word-break: break-all;
        }}
        .stat-box {{
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
            text-align: center;
        }}
        .stat-label {{
            color: #888;
            font-size: 0.9em;
            margin-bottom: 8px;
        }}
        .stat-value {{
            color: #4a9eff;
            font-size: 2em;
            font-weight: bold;
        }}
        .status-online {{
            color: #4ade80;
            font-weight: bold;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .listeners {{
            list-style: none;
            padding: 0;
            margin: 0;
        }}
        .listeners li {{
            padding: 4px 0;
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 10px;
        }}
        th {{
            background: #252525;
            color: #4a9eff;
            padding: 12px;
            text-align: left;
            font-weight: 600;
            border-bottom: 2px solid #333;
        }}
        td {{
            padding: 10px 12px;
            border-bottom: 1px solid #2a2a2a;
        }}
        tr:hover {{
            background: #1f1f1f;
        }}
        code {{
            background: #252525;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 0.9em;
        }}
        .blocks-container {{
            max-height: 600px;
            overflow-y: auto;
        }}
        .blocks-container::-webkit-scrollbar {{
            width: 8px;
        }}
        .blocks-container::-webkit-scrollbar-track {{
            background: #1a1a1a;
        }}
        .blocks-container::-webkit-scrollbar-thumb {{
            background: #333;
            border-radius: 4px;
        }}
        .blocks-container::-webkit-scrollbar-thumb:hover {{
            background: #444;
        }}
    </style>
    <script>
        // Auto-refresh every 10 seconds
        setTimeout(function() {{
            location.reload();
        }}, 10000);
    </script>
</head>
<body>
    <div class="header">
        <h1>🟢 Modality Network Node</h1>
        <p class="status-online">Status: ONLINE</p>
    </div>
    
    <div class="status-grid">
        <div class="stat-box">
            <div class="stat-label">Connected Peers</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Block Height</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Blocks Mined by Node</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Current Difficulty</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Current Epoch</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Cumulative Difficulty</div>
            <div class="stat-value">{}</div>
        </div>
    </div>
    
    <div class="status-card">
        <h2>Node Information</h2>
        <div class="status-item">
            <span class="label">Peer ID:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Listeners:</span>
            <div class="value">
                <ul class="listeners">
                    {}
                </ul>
            </div>
        </div>
    </div>

    <div class="status-card">
        <h2>Blockchain Status</h2>
        <div class="status-item">
            <span class="label">Current Round:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Latest Block Round:</span>
            <span class="value">{}</span>
        </div>
    </div>

    <div class="status-card">
        <h2>Genesis Block (Block 0)</h2>
        {}
    </div>

    <div class="status-card">
        <h2>Connected Peers</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Peer ID</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>

    <div class="status-card">
        <h2>Recent Blocks (Last 80)</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Index</th>
                        <th>Epoch</th>
                        <th>Hash</th>
                        <th>Nominee</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>

    <div class="status-card">
        <h2>First 40 Blocks</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Index</th>
                        <th>Epoch</th>
                        <th>Hash</th>
                        <th>Nominee</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>

    <div class="status-card">
        <p style="text-align: center; color: #666; font-size: 0.9em;">
            Page auto-refreshes every 10 seconds
        </p>
    </div>
</body>
</html>"#,
        connected_peers,
        total_miner_blocks,
        blocks_mined_by_node,
        current_difficulty,
        current_epoch,
        cumulative_difficulty,
        peerid,
        listeners
            .iter()
            .map(|l| format!("<li>{}</li>", l))
            .collect::<Vec<_>>()
            .join("\n                    "),
        current_round,
        latest_round,
        block_0_html,
        peers_html,
        blocks_html,
        first_blocks_html,
    );

    Ok(warp::reply::html(html))
}

