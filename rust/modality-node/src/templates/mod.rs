//! HTML templates for the modality-node.
//!
//! This module provides the status page HTML template and helper functions
//! for rendering dynamic content.

use crate::status_snapshot::{
    BlockStatus, EpochNominees, GenesisBlock, NodeStatus, PeerStatus, PrefixCertStatus, RoundStatus,
};

/// Miner status app. Contract exploration is a switch inside this page.
pub const STATUS_TEMPLATE: &str = include_str!("status.html");

/// `on` for an observer (the public explorer), `off` for miners and other roles.
pub fn explore_contracts_default(status: &NodeStatus) -> &'static str {
    let role = status.role.to_ascii_lowercase();
    let display = status.role_display.to_ascii_lowercase();
    if role == "observer" || display == "observer" {
        "on"
    } else {
        "off"
    }
}

pub use crate::status_snapshot::display_node_role;

pub fn render_role_chips(active: &[&str]) -> String {
    ["Miner", "Sequencer", "Validator"]
        .iter()
        .map(|role| {
            let on = if active.contains(role) { " on" } else { "" };
            format!(r#"<span class="role-chip{on}">{role}</span>"#)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn truncate_middle(value: &str, keep: usize) -> String {
    if value.len() > keep * 2 {
        format!("{}...{}", &value[..keep], &value[value.len() - keep..])
    } else {
        value.to_string()
    }
}

/// Template for a block row in the blocks table
pub fn render_block_row(
    index: u64,
    epoch: u64,
    hash: &str,
    nominated_peer_id: &str,
    timestamp: i64,
    time_delta: &str,
) -> String {
    format!(
        r#"<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td class="timestamp" data-timestamp="{}" onclick="toggleTimestamp(this)" style="cursor: pointer;" title="Click to toggle local time">{}</td><td>{}</td></tr>"#,
        index,
        epoch,
        truncate_middle(hash, 8),
        truncate_middle(nominated_peer_id, 10),
        timestamp,
        timestamp,
        time_delta
    )
}

fn render_block_status_row(block: &BlockStatus) -> String {
    render_block_row(
        block.index,
        block.epoch,
        &block.hash,
        &block.nominee,
        block.timestamp,
        &block.time_delta,
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
            r#"<tr><td><code>{}</code></td><td><a href="{}" target="_blank">Status</a></td></tr>"#,
            peer_id, url
        )
    } else {
        format!("<tr><td><code>{}</code></td><td>-</td></tr>", peer_id)
    }
}

/// Template for a peer row with status URL and role
pub fn render_peer_row_with_metadata(
    peer_id: &str,
    status_url: Option<&str>,
    role: Option<&str>,
) -> String {
    let status_cell = if let Some(url) = status_url {
        format!(r#"<a href="{}" target="_blank">Status</a>"#, url)
    } else {
        "-".to_string()
    };

    let role_cell = role
        .map(display_node_role)
        .unwrap_or_else(|| "-".to_string());

    format!(
        r#"<tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>"#,
        peer_id, role_cell, status_cell
    )
}

fn render_peer_status_row(peer: &PeerStatus) -> String {
    render_peer_row_with_metadata(
        &peer.peer_id,
        peer.status_url.as_deref(),
        peer.role.as_deref(),
    )
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
            <span class="label">Nominated sequencer:</span>
            <span class="value"><code>{}</code></span>
        </div>"#,
        index, hash, epoch, timestamp, previous_hash, data_hash, difficulty, nominated_peer_id
    )
}

fn render_genesis(genesis: Option<&GenesisBlock>) -> String {
    match genesis {
        Some(block) => render_block_0_info(
            block.index,
            &block.hash,
            block.epoch,
            block.timestamp,
            &block.previous_hash,
            &block.data_hash,
            &block.difficulty,
            &block.nominated_peer_id,
        ),
        None => render_block_0_not_found(),
    }
}

/// Template for empty block 0
pub fn render_block_0_not_found() -> String {
    r#"<div class="status-item">
            <span class="label" style="color: var(--dim);">Block 0 not found</span>
        </div>"#
        .to_string()
}

/// Template for empty blocks table
pub fn render_empty_blocks_message() -> String {
    "<tr><td colspan='6' style='text-align: center; padding: 20px; color: var(--dim);'>No blocks yet</td></tr>".to_string()
}

/// Template for empty peers table
pub fn render_empty_peers_message() -> String {
    "<tr><td colspan='3' style='text-align: center; padding: 20px; color: var(--dim);'>No connected peers</td></tr>".to_string()
}

pub fn render_empty_sequencer_committee() -> String {
    "<tr><td style='text-align: center; padding: 20px; color: var(--dim);'>Sequencer committee is empty until mining epoch 2 (N−2 lookback).</td></tr>".to_string()
}

/// Template for epoch nominees section
pub fn render_epoch_nominees_section(epoch: u64, nominees_html: &str) -> String {
    format!(
        r#"
    <div class="status-card">
        <h2>Epoch {} Sequencer Nominees (Shuffled Order)</h2>
        <div class="blocks-container">
            <table>
                <thead>
                    <tr>
                        <th>Shuffle Rank</th>
                        <th>Block Index</th>
                        <th>Nominating Block Hash</th>
                        <th>Nominated sequencer</th>
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
    format!(
        "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
        rank,
        block_idx,
        truncate_middle(block_hash, 8),
        truncate_middle(peer_id, 10)
    )
}

fn render_epoch_nominees_sections(epochs: &[EpochNominees]) -> String {
    if epochs.is_empty() {
        return String::new();
    }
    epochs
        .iter()
        .map(|epoch| {
            let rows = epoch
                .nominees
                .iter()
                .map(|n| render_nominee_row(n.rank, n.block_index, &n.block_hash, &n.peer_id))
                .collect::<Vec<_>>()
                .join("\n                    ");
            render_epoch_nominees_section(epoch.epoch, &rows)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Template for finalized rounds section
pub fn render_finalized_rounds_section(rounds_html: &str) -> String {
    format!(
        r#"
    <div class="status-card">
        <h2>Recently Finalized Shoal Rounds</h2>
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
        "Finalized" => "var(--ok)",
        "Partial" => "var(--warn)",
        _ => "var(--dim)",
    };

    format!(
        r#"<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td><td style="color: {};">{}</td></tr>"#,
        round_id, certified_count, total_count, completion_pct, status_color, status
    )
}

/// Template for empty finalized rounds
pub fn render_empty_finalized_rounds() -> String {
    "<tr><td colspan='5' style='text-align: center; padding: 20px; color: var(--dim);'>No finalized rounds yet</td></tr>".to_string()
}

fn render_finalized_rounds(rounds: &[RoundStatus]) -> String {
    if rounds.is_empty() {
        return render_finalized_rounds_section(&render_empty_finalized_rounds());
    }
    let rows = rounds
        .iter()
        .map(|round| {
            let completion_pct = if round.total_count > 0 {
                (round.certified_count as f32 / round.total_count as f32) * 100.0
            } else {
                0.0
            };
            render_finalized_round_row(
                round.round_id,
                round.certified_count,
                round.total_count,
                completion_pct,
                round.status,
            )
        })
        .collect::<Vec<_>>()
        .join("\n                    ");
    render_finalized_rounds_section(&rows)
}

pub fn render_empty_validators() -> String {
    r#"<p class="tab-copy">No named validators on this network. Validators attest contract prefixes for dest REPOST and dest RECV; they are not sequencers and not the language verifier.</p>"#.to_string()
}

pub fn render_named_validators(peers: &[String]) -> String {
    if peers.is_empty() {
        return render_empty_validators();
    }
    let rows = peers
        .iter()
        .map(|peer| format!("<tr><td><code>{}</code></td></tr>", peer))
        .collect::<Vec<_>>()
        .join("\n                    ");
    format!(
        r#"<div class="blocks-container">
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
        </div>"#,
        rows
    )
}

pub fn render_empty_prefix_certs() -> String {
    "<tr><td colspan='4' style='text-align: center; padding: 20px; color: var(--dim);'>No prefix certs yet</td></tr>".to_string()
}

pub fn render_prefix_cert_row(cert: &PrefixCertStatus) -> String {
    format!(
        "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td><code>{}</code></td></tr>",
        truncate_middle(&cert.source_contract, 10),
        truncate_middle(&cert.through_commit, 8),
        truncate_middle(&cert.validator_peer_id, 10),
        truncate_middle(&cert.prefix_digest, 8)
    )
}

fn render_prefix_certs(certs: &[PrefixCertStatus]) -> String {
    if certs.is_empty() {
        return render_empty_prefix_certs();
    }
    certs
        .iter()
        .map(render_prefix_cert_row)
        .collect::<Vec<_>>()
        .join("\n                    ")
}

fn render_block_rows(blocks: &[BlockStatus]) -> String {
    if blocks.is_empty() {
        return render_empty_blocks_message();
    }
    blocks
        .iter()
        .map(render_block_status_row)
        .collect::<Vec<_>>()
        .join("\n                    ")
}

fn render_peer_rows(peers: &[PeerStatus]) -> String {
    if peers.is_empty() {
        return render_empty_peers_message();
    }
    peers
        .iter()
        .map(render_peer_status_row)
        .collect::<Vec<_>>()
        .join("\n                    ")
}

fn render_sequencer_committee(peers: &[String]) -> String {
    if peers.is_empty() {
        return render_empty_sequencer_committee();
    }
    peers
        .iter()
        .map(|peer| format!("<tr><td><code>{}</code></td></tr>", peer))
        .collect::<Vec<_>>()
        .join("\n                    ")
}

pub fn render_status_from_snapshot(status: &NodeStatus) -> String {
    let nomination = status
        .sequencer_nomination_epoch
        .map(|e| e.to_string())
        .unwrap_or_else(|| "—".to_string());
    let vars = StatusPageVars {
        refresh_interval: crate::constants::STATUS_PAGE_REFRESH_SECS,
        connected_peers: status.connected_peers,
        total_miner_blocks: status.total_miner_blocks,
        cumulative_difficulty: status.cumulative_difficulty,
        peerid: status.peerid.clone(),
        network_name: status.network_name.clone(),
        role_display: status.role_display.clone(),
        role_chips_html: render_role_chips(&status.active_roles),
        hybrid_label: status.hybrid_label().to_string(),
        listeners_html: status
            .listeners
            .iter()
            .map(|l| render_listener_item(l))
            .collect::<Vec<_>>()
            .join("\n                    "),
        current_round: status.current_round,
        block_0_html: render_genesis(status.genesis.as_ref()),
        peers_html: render_peer_rows(&status.peers),
        blocks_mined_by_node: status.blocks_mined_by_node,
        current_difficulty: status.current_difficulty.clone(),
        miner_hashrate: status.miner_hashrate.clone(),
        network_hashrate: status.network_hashrate.clone(),
        recent_blocks_count: status.recent_blocks.len(),
        blocks_html: render_block_rows(&status.recent_blocks),
        first_blocks_count: status.first_blocks.len(),
        first_blocks_html: render_block_rows(&status.first_blocks),
        current_epoch: status.current_epoch,
        sequencer_nomination_epoch: nomination,
        sequencer_committee_html: render_sequencer_committee(&status.sequencer_committee),
        epoch_nominees_sections: render_epoch_nominees_sections(&status.epoch_nominees),
        finalized_rounds_section: render_finalized_rounds(&status.finalized_rounds),
        named_validator_count: status.named_validators.len(),
        validator_min_stake: status.validator_min_stake,
        validator_qc: format!(
            "⌈{}n/{}⌉",
            status.validator_qc_numerator, status.validator_qc_denominator
        ),
        dest_apply_requires_cert: if status.dest_apply_requires_cert {
            "required".to_string()
        } else {
            "not required".to_string()
        },
        pending_prefix_cert_requests: status.pending_prefix_cert_requests,
        named_validators_html: render_named_validators(&status.named_validators),
        prefix_certs_html: render_prefix_certs(&status.recent_prefix_certs),
        explore_default: explore_contracts_default(status).to_string(),
    };
    render_status_page(vars)
}

/// Render the complete status page by replacing placeholders in the template
pub fn render_status_page(vars: StatusPageVars) -> String {
    STATUS_TEMPLATE
        .replace("{refresh_interval}", &vars.refresh_interval.to_string())
        .replace("{connected_peers}", &vars.connected_peers.to_string())
        .replace("{total_miner_blocks}", &vars.total_miner_blocks.to_string())
        .replace(
            "{cumulative_difficulty}",
            &vars.cumulative_difficulty.to_string(),
        )
        .replace("{peerid}", &vars.peerid)
        .replace("{network_name}", &vars.network_name)
        .replace("{role_display}", &vars.role_display)
        .replace("{role_chips_html}", &vars.role_chips_html)
        .replace("{hybrid_label}", &vars.hybrid_label)
        .replace("{listeners_html}", &vars.listeners_html)
        .replace("{current_round}", &vars.current_round.to_string())
        .replace("{block_0_html}", &vars.block_0_html)
        .replace("{peers_html}", &vars.peers_html)
        .replace(
            "{blocks_mined_by_node}",
            &vars.blocks_mined_by_node.to_string(),
        )
        .replace("{current_difficulty}", &vars.current_difficulty)
        .replace("{miner_hashrate}", &vars.miner_hashrate)
        .replace("{network_hashrate}", &vars.network_hashrate)
        .replace(
            "{recent_blocks_count}",
            &vars.recent_blocks_count.to_string(),
        )
        .replace("{blocks_html}", &vars.blocks_html)
        .replace("{first_blocks_count}", &vars.first_blocks_count.to_string())
        .replace("{first_blocks_html}", &vars.first_blocks_html)
        .replace("{current_epoch}", &vars.current_epoch.to_string())
        .replace(
            "{sequencer_nomination_epoch}",
            &vars.sequencer_nomination_epoch,
        )
        .replace("{sequencer_committee_html}", &vars.sequencer_committee_html)
        .replace("{epoch_nominees_sections}", &vars.epoch_nominees_sections)
        .replace("{finalized_rounds_section}", &vars.finalized_rounds_section)
        .replace(
            "{named_validator_count}",
            &vars.named_validator_count.to_string(),
        )
        .replace(
            "{validator_min_stake}",
            &vars.validator_min_stake.to_string(),
        )
        .replace("{validator_qc}", &vars.validator_qc)
        .replace("{dest_apply_requires_cert}", &vars.dest_apply_requires_cert)
        .replace(
            "{pending_prefix_cert_requests}",
            &vars.pending_prefix_cert_requests.to_string(),
        )
        .replace("{named_validators_html}", &vars.named_validators_html)
        .replace("{prefix_certs_html}", &vars.prefix_certs_html)
        .replace("{explore_default}", &vars.explore_default)
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
    pub role_display: String,
    pub role_chips_html: String,
    pub hybrid_label: String,
    pub listeners_html: String,
    pub current_round: u64,
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
    pub sequencer_nomination_epoch: String,
    pub sequencer_committee_html: String,
    pub epoch_nominees_sections: String,
    pub finalized_rounds_section: String,
    pub named_validator_count: usize,
    pub validator_min_stake: u64,
    pub validator_qc: String,
    pub dest_apply_requires_cert: String,
    pub pending_prefix_cert_requests: usize,
    pub named_validators_html: String,
    pub prefix_certs_html: String,
    pub explore_default: String,
}

fn sample_vars() -> StatusPageVars {
    StatusPageVars {
        refresh_interval: 10,
        connected_peers: 4,
        total_miner_blocks: 170,
        cumulative_difficulty: 1098,
        peerid: "12D3KooWBGR3m1JmVFm2aZYR7TZXicjA7HSVSWi2fama5cPpgQiX".to_string(),
        network_name: "TestNet".to_string(),
        role_display: "Miner".to_string(),
        role_chips_html: render_role_chips(&["Miner"]),
        hybrid_label: "on (sequencers from epoch N−2)".to_string(),
        listeners_html: "<li>/ip4/0.0.0.0/tcp/4040/ws</li>".to_string(),
        current_round: 0,
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
        sequencer_nomination_epoch: "2".to_string(),
        sequencer_committee_html: "<tr><td>seq</td></tr>".to_string(),
        epoch_nominees_sections: "<div>Epoch data</div>".to_string(),
        finalized_rounds_section: "<div>Finalized rounds</div>".to_string(),
        named_validator_count: 0,
        validator_min_stake: 0,
        validator_qc: "⌈2n/3⌉".to_string(),
        dest_apply_requires_cert: "not required".to_string(),
        pending_prefix_cert_requests: 0,
        named_validators_html: render_empty_validators(),
        prefix_certs_html: render_empty_prefix_certs(),
        explore_default: "off".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_snapshot::sample_status;

    #[test]
    fn display_node_role_maps_protocol_names() {
        assert_eq!(display_node_role("miner"), "Miner");
        assert_eq!(display_node_role("hybrid"), "Miner+Sequencer");
        assert_eq!(display_node_role("Miner+Validator"), "Miner+Sequencer");
        assert_eq!(display_node_role("validator"), "Sequencer");
        assert_eq!(display_node_role("sequencer"), "Sequencer");
        assert_eq!(display_node_role("contract-validator"), "Validator");
        assert_eq!(display_node_role("observer"), "Observer");
    }

    #[test]
    fn test_render_status_page_converts_double_braces() {
        let html = render_status_page(sample_vars());

        assert!(
            html.contains("body {"),
            "CSS should have single opening brace for body"
        );
        assert!(
            html.contains(".status-card {"),
            "CSS class selectors should have single braces"
        );

        let style_start = html.find("<style>").expect("Should have style tag");
        let style_end = html
            .find("</style>")
            .expect("Should have closing style tag");
        let style_content = &html[style_start..style_end];

        assert!(
            !style_content.contains("{{"),
            "CSS should NOT have double opening braces"
        );
        assert!(
            !style_content.contains("}}"),
            "CSS should NOT have double closing braces"
        );

        assert!(html.contains("TestNet"));
        assert!(html.contains("12D3KooWBGR3m1JmVFm2aZYR7TZXicjA7HSVSWi2fama5cPpgQiX"));
        assert!(html.contains("170"));
        assert!(html.contains("Modality Network"));
        assert!(!html.contains("Modal Money"));
        assert!(html.contains("Not mainnet"));
        assert!(html.contains("stat-label\">Epoch"));
        assert!(html.contains("data-tab=\"sequencers\""));
        assert!(html.contains("data-tab=\"validators\""));
        assert!(html.contains("data-tab=\"miners\""));
        assert!(html.contains("data-active-tab"));
        assert!(html.contains("refreshStatus"));
        assert!(html.contains("/status.json"));
        assert!(html.contains("Explore contracts"));
        assert!(html.contains("Paste a contract ID"));
        assert!(html.contains("data-tab=\"contracts\""));
        assert!(html.contains("data-explore=\"off\">"));
        assert!(html.contains("var explore = 'off';"));
        assert!(html.contains("id=\"explore-toggle\""));
    }

    #[test]
    fn rendered_scripts_have_balanced_braces() {
        let html = render_status_page(sample_vars());
        let mut rest = html.as_str();
        let mut found = false;
        while let Some(start) = rest.find("<script>") {
            found = true;
            rest = &rest[start + "<script>".len()..];
            let end = rest.find("</script>").expect("script should close");
            let script = &rest[..end];
            let open = script.matches('{').count();
            let close = script.matches('}').count();
            assert_eq!(open, close, "unbalanced braces:\n{script}");
            rest = &rest[end..];
        }
        assert!(found);
    }

    #[test]
    fn empty_named_validators_explains_the_role() {
        let html = render_named_validators(&[]);
        assert!(html.contains("No named validators on this network"));
        assert!(!html.contains("language verifier") || html.contains("not the language verifier"));
    }

    #[test]
    fn named_validators_list_peer_ids() {
        let html = render_named_validators(&["12D3KooWnamed".to_string()]);
        assert!(html.contains("12D3KooWnamed"));
        assert!(!html.contains("No named validators on this network"));
    }

    #[test]
    fn snapshot_render_has_three_role_tabs() {
        let mut status = sample_status();
        status.network_name = "testnet".into();
        status.named_validators = vec![];
        let html = render_status_from_snapshot(&status);
        assert!(html.contains("Modality Network"));
        assert!(html.contains("<h1>testnet</h1>"));
        assert!(!html.contains("Modal Money"));
        assert!(html.contains("switchTab('sequencers')"));
        assert!(html.contains("switchTab('validators')"));
        assert!(html.contains("No named validators on this network"));
    }

    #[test]
    fn miner_snapshot_leaves_contract_exploration_off() {
        let html = render_status_from_snapshot(&sample_status());
        assert!(html.contains("data-explore=\"off\">"));
        assert!(html.contains("var explore = 'off';"));
        assert!(!html.contains("{explore_default}"));
    }

    #[test]
    fn observer_snapshot_turns_contract_exploration_on() {
        let mut status = sample_status();
        status.role = "observer".into();
        status.role_display = "Observer".into();
        status.active_roles = vec![];
        let html = render_status_from_snapshot(&status);
        assert!(html.contains("data-explore=\"on\">"));
        assert!(html.contains("var explore = 'on';"));
    }

    #[test]
    fn snapshot_render_lists_named_validators() {
        let mut status = sample_status();
        status.named_validators = vec!["12D3KooWval".into()];
        let html = render_status_from_snapshot(&status);
        assert!(html.contains("12D3KooWval"));
        assert!(!html.contains("No named validators on this network"));
    }
}
