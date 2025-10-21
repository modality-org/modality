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
    
    // Create a map of block index to block for quick parent lookup
    let block_map: std::collections::HashMap<u64, &MinerBlock> = miner_blocks
        .iter()
        .map(|block| (block.index, block))
        .collect();
    
    // Calculate epoch nominees with shuffle order for previous epochs
    const BLOCKS_PER_EPOCH: u64 = 40;
    let mut epoch_nominees_data: Vec<(u64, Vec<(usize, String, String, u64)>)> = Vec::new();
    
    if current_epoch > 0 {
        // Show up to 5 previous epochs
        let epochs_to_show = std::cmp::min(5, current_epoch);
        for epoch_offset in 1..=epochs_to_show {
            let epoch = current_epoch - epoch_offset;
            let epoch_start = epoch * BLOCKS_PER_EPOCH;
            let epoch_end = epoch_start + BLOCKS_PER_EPOCH;
            
            // Get all blocks from this epoch
            let epoch_blocks: Vec<&MinerBlock> = miner_blocks
                .iter()
                .filter(|b| b.index >= epoch_start && b.index < epoch_end)
                .collect();
            
            // Only process complete epochs
            if epoch_blocks.len() == BLOCKS_PER_EPOCH as usize {
                // Calculate XOR seed from all nonces
                let mut seed = 0u64;
                for block in &epoch_blocks {
                    if let Ok(nonce) = block.nonce.parse::<u128>() {
                        seed ^= nonce as u64;
                    }
                }
                
                // Get shuffled indices using Fisher-Yates
                let shuffled_indices = modality_utils::shuffle::fisher_yates_shuffle(seed, epoch_blocks.len());
                
                // Map shuffled indices to (shuffle_rank, block_hash, nominated_peer_id, block_index)
                let shuffled_nominees: Vec<(usize, String, String, u64)> = shuffled_indices
                    .into_iter()
                    .enumerate()
                    .map(|(rank, original_idx)| {
                        let block = epoch_blocks[original_idx];
                        (rank, block.hash.clone(), block.nominated_peer_id.clone(), block.index)
                    })
                    .collect();
                
                epoch_nominees_data.push((epoch, shuffled_nominees));
            }
        }
    }
    
    drop(ds);

    // Build blocks table HTML for recent blocks (last 80)
    let blocks_html = if recent_blocks.is_empty() {
        "<tr><td colspan='6' style='text-align: center; padding: 20px; color: #666;'>No blocks yet</td></tr>".to_string()
    } else {
        recent_blocks
            .iter()
            .map(|block| {
                // Calculate time delta from parent block
                let time_delta = if block.index == 0 {
                    "-".to_string()
                } else if let Some(parent) = block_map.get(&(block.index - 1)) {
                    (block.timestamp - parent.timestamp).to_string()
                } else {
                    "N/A".to_string()
                };
                
                format!(
                    "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
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
                    block.timestamp,
                    time_delta
                )
            })
            .collect::<Vec<_>>()
            .join("\n                    ")
    };

    // Build blocks table HTML for first 40 blocks
    let first_blocks_html = if first_blocks.is_empty() {
        "<tr><td colspan='6' style='text-align: center; padding: 20px; color: #666;'>No blocks yet</td></tr>".to_string()
    } else {
        first_blocks
            .iter()
            .map(|block| {
                // Calculate time delta from parent block
                let time_delta = if block.index == 0 {
                    "-".to_string()
                } else if let Some(parent) = block_map.get(&(block.index - 1)) {
                    (block.timestamp - parent.timestamp).to_string()
                } else {
                    "N/A".to_string()
                };
                
                format!(
                    "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
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
                    block.timestamp,
                    time_delta
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

    // Build epoch nominees HTML sections
    let epoch_nominees_sections = if epoch_nominees_data.is_empty() {
        String::new()
    } else {
        epoch_nominees_data
            .iter()
            .map(|(epoch, nominees)| {
                let nominees_html = nominees
                    .iter()
                    .map(|(rank, block_hash, peer_id, block_idx)| {
                        let truncated_hash = if block_hash.len() > 16 {
                            format!("{}...{}", &block_hash[..8], &block_hash[block_hash.len()-8..])
                        } else {
                            block_hash.clone()
                        };
                        let truncated_peer = if peer_id.len() > 20 {
                            format!("{}...{}", &peer_id[..10], &peer_id[peer_id.len()-10..])
                        } else {
                            peer_id.clone()
                        };
                        format!(
                            "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                            rank + 1, // Display rank as 1-indexed
                            block_idx,
                            truncated_hash,
                            truncated_peer
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n                    ");

                format!(
                    r#"
    <div class="status-card">
        <h2>Epoch {} Nominees (Shuffled Order)</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Shuffle Rank</th>
                        <th>Block Index</th>
                        <th>Nominating Block Hash</th>
                        <th>Nominated Peer</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>"#,
                    epoch,
                    nominees_html
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
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
        
        /* Tab Styles */
        .tabs {{
            display: flex;
            gap: 10px;
            border-bottom: 2px solid #333;
            margin-bottom: 30px;
        }}
        .tab {{
            padding: 12px 24px;
            background: #1a1a1a;
            border: 1px solid #333;
            border-bottom: none;
            border-radius: 8px 8px 0 0;
            cursor: pointer;
            color: #888;
            font-weight: 600;
            transition: all 0.3s ease;
        }}
        .tab:hover {{
            background: #252525;
            color: #e0e0e0;
        }}
        .tab.active {{
            background: #0f0f0f;
            color: #4a9eff;
            border-bottom: 2px solid #4a9eff;
            margin-bottom: -2px;
        }}
        .tab-content {{
            display: none;
        }}
        .tab-content.active {{
            display: block;
        }}
    </style>
    <script>
        // Tab switching
        function switchTab(tabName) {{
            // Hide all tab contents
            document.querySelectorAll('.tab-content').forEach(function(content) {{
                content.classList.remove('active');
            }});
            
            // Deactivate all tabs
            document.querySelectorAll('.tab').forEach(function(tab) {{
                tab.classList.remove('active');
            }});
            
            // Show selected tab content
            document.getElementById(tabName + '-content').classList.add('active');
            
            // Activate selected tab
            document.querySelector('[data-tab="' + tabName + '"]').classList.add('active');
            
            // Save active tab to localStorage
            localStorage.setItem('activeTab', tabName);
        }}
        
        // Restore active tab on page load
        window.addEventListener('DOMContentLoaded', function() {{
            const savedTab = localStorage.getItem('activeTab') || 'overview';
            switchTab(savedTab);
        }});
        
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
    
    <!-- Tabs Navigation -->
    <div class="tabs">
        <div class="tab active" data-tab="overview" onclick="switchTab('overview')">Overview</div>
        <div class="tab" data-tab="mining" onclick="switchTab('mining')">Mining</div>
        <div class="tab" data-tab="sequencing" onclick="switchTab('sequencing')">Sequencing</div>
    </div>
    
    <!-- Overview Tab -->
    <div id="overview-content" class="tab-content active">
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
    </div>
    
    <!-- Mining Tab -->
    <div id="mining-content" class="tab-content">
        <div class="status-grid">
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
                            <th>Time Delta (s)</th>
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
                            <th>Time Delta (s)</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
        </div>
    </div>
    
    <!-- Sequencing Tab -->
    <div id="sequencing-content" class="tab-content">
        <div class="status-grid">
            <div class="stat-box">
                <div class="stat-label">Current Epoch</div>
                <div class="stat-value">{}</div>
            </div>
            <div class="stat-box">
                <div class="stat-label">Block Height</div>
                <div class="stat-value">{}</div>
            </div>
            <div class="stat-box">
                <div class="stat-label">Completed Epochs</div>
                <div class="stat-value">{}</div>
            </div>
        </div>

        {}
    </div>

    <div class="status-card">
        <p style="text-align: center; color: #666; font-size: 0.9em;">
            Page auto-refreshes every 10 seconds
        </p>
    </div>
</body>
</html>"#,
        // Overview tab
        connected_peers,
        total_miner_blocks,
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
        // Mining tab
        total_miner_blocks,
        blocks_mined_by_node,
        current_difficulty,
        blocks_html,
        first_blocks_html,
        // Sequencing tab
        current_epoch,
        total_miner_blocks,
        current_epoch, // completed epochs (same as current epoch for now)
        epoch_nominees_sections,
    );

    Ok(warp::reply::html(html))
}

