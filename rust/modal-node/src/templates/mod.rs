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

/// Template for a peer row with status URL
pub fn render_peer_row_with_url(peer_id: &str, status_url: Option<&str>) -> String {
    if let Some(url) = status_url {
        format!(
            r#"<tr><td><code>{}</code></td><td><a href="{}" target="_blank" style="color: #4ade80; text-decoration: none;">🔗 Status</a></td></tr>"#,
            peer_id, url
        )
    } else {
        format!("<tr><td><code>{}</code></td><td>-</td></tr>", peer_id)
    }
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

/// Template for finalized rounds section
pub fn render_finalized_rounds_section(rounds_html: &str) -> String {
    format!(
        r#"
    <div class="status-card">
        <h2>Recently Finalized Rounds</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Round</th>
                        <th>Certified Blocks</th>
                        <th>Total Blocks</th>
                        <th>Completion %</th>
                        <th>Status</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>"#,
        rounds_html
    )
}

/// Template for a finalized round row
pub fn render_finalized_round_row(
    round_id: u64,
    certified_count: usize,
    total_count: usize,
    completion_pct: f32,
    status: &str,
) -> String {
    let status_color = match status {
        "Finalized" => "#4ade80",
        "Partial" => "#fbbf24",
        _ => "#888",
    };
    
    format!(
        r#"<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td><td style="color: {};">{}</td></tr>"#,
        round_id,
        certified_count,
        total_count,
        completion_pct,
        status_color,
        status
    )
}

/// Template for empty finalized rounds
pub fn render_empty_finalized_rounds() -> String {
    "<tr><td colspan='5' style='text-align: center; padding: 20px; color: #666;'>No finalized rounds yet</td></tr>".to_string()
}

/// Render the complete status page by replacing placeholders in the template
pub fn render_status_page(vars: StatusPageVars) -> String {
    STATUS_TEMPLATE
        .replace("{refresh_interval}", &vars.refresh_interval.to_string())
        .replace("{connected_peers}", &vars.connected_peers.to_string())
        .replace("{total_miner_blocks}", &vars.total_miner_blocks.to_string())
        .replace("{cumulative_difficulty}", &vars.cumulative_difficulty.to_string())
        .replace("{peerid}", &vars.peerid)
        .replace("{network_name}", &vars.network_name)
        .replace("{role}", &vars.role)
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
        .replace("{finalized_rounds_section}", &vars.finalized_rounds_section)
        // Convert double braces back to single braces for CSS/JavaScript
        .replace("{{", "{")
        .replace("}}", "}")
}

/// Variables for rendering the status page template
pub struct StatusPageVars {
    pub refresh_interval: u64,
    pub connected_peers: usize,
    pub total_miner_blocks: usize,
    pub cumulative_difficulty: u128,
    pub peerid: String,
    pub network_name: String,
    pub role: String,
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
    pub finalized_rounds_section: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_status_page_converts_double_braces() {
        let vars = StatusPageVars {
            refresh_interval: 10,
            connected_peers: 4,
            total_miner_blocks: 170,
            cumulative_difficulty: 1098,
            peerid: "12D3KooWBGR3m1JmVFm2aZYR7TZXicjA7HSVSWi2fama5cPpgQiX".to_string(),
            network_name: "TestNet".to_string(),
            role: "miner".to_string(),
            listeners_html: "<li>/ip4/0.0.0.0/tcp/4040/ws</li>".to_string(),
            current_round: 0,
            latest_round: 0,
            block_0_html: "<div>Test</div>".to_string(),
            peers_html: "<tr><td>Test Peer</td></tr>".to_string(),
            blocks_mined_by_node: 9,
            current_difficulty: "12".to_string(),
            miner_hashrate: "0".to_string(),
            network_hashrate: "64.75".to_string(),
            recent_blocks_count: 80,
            blocks_html: "<tr><td>167</td></tr>".to_string(),
            first_blocks_count: 10,
            first_blocks_html: "<tr><td>0</td></tr>".to_string(),
            current_epoch: 4,
            completed_epochs: 4,
            epoch_nominees_sections: "<div>Epoch data</div>".to_string(),
            finalized_rounds_section: "<div>Finalized rounds</div>".to_string(),
        };

        let html = render_status_page(vars);
        
        // Check that CSS has single braces (valid CSS)
        assert!(html.contains("body {"), "CSS should have single opening brace for body");
        assert!(html.contains("color: #e0e0e0;"), "CSS properties should be present");
        assert!(html.contains(".status-card {"), "CSS class selectors should have single braces");
        
        // Check that double braces in CSS were converted
        // We should not find {{ or }} in the style section
        let style_start = html.find("<style>").expect("Should have style tag");
        let style_end = html.find("</style>").expect("Should have closing style tag");
        let style_content = &html[style_start..style_end];
        
        assert!(!style_content.contains("{{"), "CSS should NOT have double opening braces");
        assert!(!style_content.contains("}}"), "CSS should NOT have double closing braces");
        
        // Check that placeholders were replaced
        assert!(html.contains("TestNet"), "Network name placeholder should be replaced");
        assert!(html.contains("12D3KooWBGR3m1JmVFm2aZYR7TZXicjA7HSVSWi2fama5cPpgQiX"), "Peer ID placeholder should be replaced");
        assert!(html.contains("170"), "Block count placeholder should be replaced");
    }
}

