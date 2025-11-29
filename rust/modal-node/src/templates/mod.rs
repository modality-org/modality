//! HTML templates for the modal-node.
//!
//! This module provides the status page HTML template and helper functions
//! for rendering dynamic content.

/// The main status page HTML template
pub const STATUS_TEMPLATE: &str = include_str!("status.html");

/// Template for a block row in the blocks table
pub fn render_block_row(
    index: u64,
    epoch: u64,
    hash: &str,
    nominated_peer_id: &str,
    timestamp: i64,
    time_delta: &str,
) -> String {
    let truncated_hash = if hash.len() > 16 {
        format!("{}...{}", &hash[..8], &hash[hash.len()-8..])
    } else {
        hash.to_string()
    };
    
    let truncated_peer = if nominated_peer_id.len() > 20 {
        format!("{}...{}", &nominated_peer_id[..10], &nominated_peer_id[nominated_peer_id.len()-10..])
    } else {
        nominated_peer_id.to_string()
    };
    
    format!(
        r#"<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td class="timestamp" data-timestamp="{}" onclick="toggleTimestamp(this)" style="cursor: pointer;" title="Click to toggle local time">{}</td><td>{}</td></tr>"#,
        index,
        epoch,
        truncated_hash,
        truncated_peer,
        timestamp,
        timestamp,
        time_delta
    )
}

/// Template for a peer row in the peers table
pub fn render_peer_row(peer_id: &str) -> String {
    format!("<tr><td><code>{}</code></td></tr>", peer_id)
}

/// Template for a listener item
pub fn render_listener_item(listener: &str) -> String {
    format!("<li>{}</li>", listener)
}

/// Template for block 0 (genesis) information
pub fn render_block_0_info(
    index: u64,
    hash: &str,
    epoch: u64,
    timestamp: i64,
    previous_hash: &str,
    data_hash: &str,
    difficulty: &str,
    nominated_peer_id: &str,
) -> String {
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
        index, hash, epoch, timestamp, previous_hash, data_hash, difficulty, nominated_peer_id
    )
}

/// Template for empty block 0
pub fn render_block_0_not_found() -> String {
    r#"<div class="status-item">
            <span class="label" style="color: #666;">Block 0 not found</span>
        </div>"#.to_string()
}

/// Template for empty blocks table
pub fn render_empty_blocks_message() -> String {
    "<tr><td colspan='6' style='text-align: center; padding: 20px; color: #666;'>No blocks yet</td></tr>".to_string()
}

/// Template for empty peers table
pub fn render_empty_peers_message() -> String {
    "<tr><td style='text-align: center; padding: 20px; color: #666;'>No connected peers</td></tr>".to_string()
}

/// Template for epoch nominees section
pub fn render_epoch_nominees_section(epoch: u64, nominees_html: &str) -> String {
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
        epoch, nominees_html
    )
}

/// Template for a nominee row
pub fn render_nominee_row(rank: usize, block_idx: u64, block_hash: &str, peer_id: &str) -> String {
    let truncated_hash = if block_hash.len() > 16 {
        format!("{}...{}", &block_hash[..8], &block_hash[block_hash.len()-8..])
    } else {
        block_hash.to_string()
    };
    let truncated_peer = if peer_id.len() > 20 {
        format!("{}...{}", &peer_id[..10], &peer_id[peer_id.len()-10..])
    } else {
        peer_id.to_string()
    };
    
    format!(
        "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
        rank,
        block_idx,
        truncated_hash,
        truncated_peer
    )
}

/// Render the complete status page by replacing placeholders in the template
pub fn render_status_page(vars: StatusPageVars) -> String {
    STATUS_TEMPLATE
        .replace("{refresh_interval}", &vars.refresh_interval.to_string())
        .replace("{connected_peers}", &vars.connected_peers.to_string())
        .replace("{total_miner_blocks}", &vars.total_miner_blocks.to_string())
        .replace("{cumulative_difficulty}", &vars.cumulative_difficulty.to_string())
        .replace("{peerid}", &vars.peerid)
        .replace("{listeners_html}", &vars.listeners_html)
        .replace("{current_round}", &vars.current_round.to_string())
        .replace("{latest_round}", &vars.latest_round.to_string())
        .replace("{block_0_html}", &vars.block_0_html)
        .replace("{peers_html}", &vars.peers_html)
        .replace("{blocks_mined_by_node}", &vars.blocks_mined_by_node.to_string())
        .replace("{current_difficulty}", &vars.current_difficulty)
        .replace("{miner_hashrate}", &vars.miner_hashrate)
        .replace("{network_hashrate}", &vars.network_hashrate)
        .replace("{recent_blocks_count}", &vars.recent_blocks_count.to_string())
        .replace("{blocks_html}", &vars.blocks_html)
        .replace("{first_blocks_count}", &vars.first_blocks_count.to_string())
        .replace("{first_blocks_html}", &vars.first_blocks_html)
        .replace("{current_epoch}", &vars.current_epoch.to_string())
        .replace("{completed_epochs}", &vars.completed_epochs.to_string())
        .replace("{epoch_nominees_sections}", &vars.epoch_nominees_sections)
}

/// Variables for rendering the status page template
pub struct StatusPageVars {
    pub refresh_interval: u64,
    pub connected_peers: usize,
    pub total_miner_blocks: usize,
    pub cumulative_difficulty: u128,
    pub peerid: String,
    pub listeners_html: String,
    pub current_round: u64,
    pub latest_round: u64,
    pub block_0_html: String,
    pub peers_html: String,
    pub blocks_mined_by_node: usize,
    pub current_difficulty: String,
    pub miner_hashrate: String,
    pub network_hashrate: String,
    pub recent_blocks_count: usize,
    pub blocks_html: String,
    pub first_blocks_count: usize,
    pub first_blocks_html: String,
    pub current_epoch: u64,
    pub completed_epochs: u64,
    pub epoch_nominees_sections: String,
}

