//! Terminal UI for a running node.
//!
//! Shown by default for foreground `node run*` when stdout is a TTY.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use modality_node::logging::LogRing;
use modality_node::status_snapshot::{NodeStatus, NodeStatusSource};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs, Wrap,
};
use ratatui::{Frame, Terminal};
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub(crate) const ACCENT: Color = Color::Rgb(74, 158, 255);
pub(crate) const GREEN: Color = Color::Rgb(74, 222, 128);
pub(crate) const MUTED: Color = Color::Rgb(136, 136, 136);
pub(crate) const BG: Color = Color::Rgb(15, 15, 15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Mining,
    Sequencing,
    Logs,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Overview, Tab::Mining, Tab::Sequencing, Tab::Logs];

    fn title(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Mining => "Mining",
            Tab::Sequencing => "Sequencing",
            Tab::Logs => "Logs",
        }
    }

    fn next(self) -> Self {
        match self {
            Tab::Overview => Tab::Mining,
            Tab::Mining => Tab::Sequencing,
            Tab::Sequencing => Tab::Logs,
            Tab::Logs => Tab::Overview,
        }
    }

    fn prev(self) -> Self {
        match self {
            Tab::Overview => Tab::Logs,
            Tab::Mining => Tab::Overview,
            Tab::Sequencing => Tab::Mining,
            Tab::Logs => Tab::Sequencing,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

pub(crate) struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, LeaveAlternateScreen);
    }
}

struct ShutdownOnExit(NodeStatusSource);

impl Drop for ShutdownOnExit {
    fn drop(&mut self) {
        self.0.request_shutdown();
    }
}

pub(crate) fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, LeaveAlternateScreen);
        original(info);
    }));
}

/// Run the node dashboard until the user quits or the node shuts down.
pub async fn run(source: NodeStatusSource, logs: LogRing) -> Result<()> {
    install_panic_hook();
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _shutdown = ShutdownOnExit(source.clone());
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let running = Arc::new(AtomicBool::new(true));
    let (key_tx, mut key_rx) = mpsc::unbounded_channel();
    let poll_running = running.clone();
    std::thread::spawn(move || {
        while poll_running.load(Ordering::Relaxed) {
            match event::poll(Duration::from_millis(200)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => {
                        if key_tx.send(key).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });

    let mut tab = Tab::Overview;
    let mut snapshot = source.snapshot().await.ok();
    let mut interval = tokio::time::interval(Duration::from_millis(750));
    let mut shutdown_rx = source.shutdown_tx.subscribe();

    loop {
        let log_lines = logs.snapshot();
        terminal.draw(|frame| draw(frame, tab, snapshot.as_ref(), &log_lines))?;

        tokio::select! {
            _ = interval.tick() => {
                match source.snapshot().await {
                    Ok(s) => snapshot = Some(s),
                    Err(e) => log::debug!("status snapshot failed: {e}"),
                }
            }
            key = key_rx.recv() => {
                let Some(key) = key else { break; };
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => tab = tab.next(),
                    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => tab = tab.prev(),
                    KeyCode::Char('1') => tab = Tab::Overview,
                    KeyCode::Char('2') => tab = Tab::Mining,
                    KeyCode::Char('3') => tab = Tab::Sequencing,
                    KeyCode::Char('4') => tab = Tab::Logs,
                    _ => {}
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }

    running.store(false, Ordering::Relaxed);
    Ok(())
}

fn draw(frame: &mut Frame, tab: Tab, status: Option<&NodeStatus>, logs: &[String]) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG).fg(Color::Rgb(224, 224, 224))),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(frame, chunks[0], status);
    draw_tabs(frame, chunks[1], tab);
    match tab {
        Tab::Overview => draw_overview(frame, chunks[2], status),
        Tab::Mining => draw_mining(frame, chunks[2], status),
        Tab::Sequencing => draw_sequencing(frame, chunks[2], status),
        Tab::Logs => draw_logs(frame, chunks[2], logs),
    }
    draw_footer(frame, chunks[3]);
}

fn draw_header(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let (role, network, peer, peers, height, epoch, round) = match status {
        Some(s) => (
            s.role.as_str(),
            s.network_name.as_str(),
            short_id(&s.peerid),
            s.connected_peers.to_string(),
            s.chain_tip.to_string(),
            s.current_epoch.to_string(),
            s.current_round.to_string(),
        ),
        None => (
            "…",
            "…",
            "starting".into(),
            "0".into(),
            "–".into(),
            "–".into(),
            "–".into(),
        ),
    };

    let line = Line::from(vec![
        Span::styled(" modality ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(role, Style::default().fg(GREEN)),
        Span::raw("  "),
        Span::styled(network, Style::default().fg(MUTED)),
        Span::raw("  peer "),
        Span::styled(peer, Style::default().fg(Color::White)),
        Span::raw("  peers "),
        Span::styled(peers, Style::default().fg(ACCENT)),
        Span::raw("  height "),
        Span::styled(height, Style::default().fg(ACCENT)),
        Span::raw("  epoch "),
        Span::styled(epoch, Style::default().fg(ACCENT)),
        Span::raw("  round "),
        Span::styled(round, Style::default().fg(ACCENT)),
    ]);

    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 51, 51)))
                .title(" node "),
        ),
        area,
    );
}

fn draw_tabs(frame: &mut Frame, area: Rect, tab: Tab) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(tab.index())
        .style(Style::default().fg(MUTED))
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    frame.render_widget(tabs, area);
}

fn draw_overview(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(area);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " epoch {}  ({}/{} blocks) ",
                    s.current_epoch,
                    s.chain_tip % s.blocks_per_epoch,
                    s.blocks_per_epoch
                )),
        )
        .gauge_style(Style::default().fg(ACCENT).bg(Color::Rgb(26, 26, 26)))
        .ratio(s.epoch_progress().clamp(0.0, 1.0));
    frame.render_widget(gauge, chunks[0]);

    let mut lines = vec![
        kv("Peer ID", &s.peerid),
        kv("Role", &s.role),
        kv("Network", &s.network_name),
        kv(
            "Hybrid",
            if s.hybrid_consensus { "on (N−2 sequencers)" } else { "off" },
        ),
        kv("Connected peers", &s.connected_peers.to_string()),
        kv("Canonical blocks", &s.total_miner_blocks.to_string()),
        kv("Difficulty", &s.current_difficulty),
        kv("Miner hashrate", &format!("{} H/s", s.miner_hashrate)),
        kv("Seq round", &s.current_round.to_string()),
    ];
    if let Some(url) = &s.status_url {
        lines.push(kv("Status page", url));
    }
    if !s.listeners.is_empty() {
        lines.push(kv("Listen", &s.listeners.join(", ")));
    }
    if s.peers.is_empty() {
        lines.push(Line::from(Span::styled(
            "No peers connected",
            Style::default().fg(MUTED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Peers",
            Style::default().fg(MUTED),
        )));
        for peer in s.peers.iter().take(8) {
            let role = peer.role.as_deref().unwrap_or("—");
            lines.push(Line::from(format!("  {}  {}", short_id(&peer.peer_id), role)));
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title(" overview ")),
        chunks[1],
    );
}

fn draw_mining(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(4)])
        .split(area);

    let stats = Paragraph::new(vec![
        kv("Height", &s.chain_tip.to_string()),
        kv("Epoch", &s.current_epoch.to_string()),
        kv("Difficulty", &s.current_difficulty),
        kv(
            "Hashrate",
            &format!(
                "miner {} H/s   network {} H/s",
                s.miner_hashrate, s.network_hashrate
            ),
        ),
        kv(
            "Blocks mined by this node",
            &s.blocks_mined_by_node.to_string(),
        ),
    ])
    .block(Block::default().borders(Borders::ALL).title(" mining "));
    frame.render_widget(stats, chunks[0]);

    let header = Row::new(["Index", "Epoch", "Hash", "Nominee"])
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    let rows = s.recent_blocks.iter().map(|b| {
        Row::new(vec![
            Cell::from(b.index.to_string()),
            Cell::from(b.epoch.to_string()),
            Cell::from(short_hash(&b.hash)),
            Cell::from(short_id(&b.nominee)),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Min(16),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" recent blocks "))
    .row_highlight_style(Style::default());
    frame.render_widget(table, chunks[1]);
}

fn draw_sequencing(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Percentage(50), Constraint::Min(4)])
        .split(area);

    let intro = if s.hybrid_consensus {
        format!(
            "Hybrid N−2 lookback  ·  sequencer round {}  ·  epoch {}",
            s.current_round, s.current_epoch
        )
    } else {
        format!("Static sequencers  ·  round {}", s.current_round)
    };
    frame.render_widget(
        Paragraph::new(intro).block(Block::default().borders(Borders::ALL).title(" consensus ")),
        chunks[0],
    );

    let header = Row::new(["Round", "Certified", "Total", "Status"])
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    let rows = s.finalized_rounds.iter().map(|r| {
        let style = if r.status == "Finalized" {
            Style::default().fg(GREEN)
        } else {
            Style::default()
        };
        Row::new(vec![
            Cell::from(r.round_id.to_string()),
            Cell::from(r.certified_count.to_string()),
            Cell::from(r.total_count.to_string()),
            Cell::from(r.status).style(style),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" finalized rounds "),
    );
    frame.render_widget(table, chunks[1]);

    let mut nominee_lines = Vec::new();
    if s.epoch_nominees.is_empty() {
        nominee_lines.push(Line::from(Span::styled(
            "Nominees appear after a complete prior epoch.",
            Style::default().fg(MUTED),
        )));
    } else {
        for epoch in &s.epoch_nominees {
            nominee_lines.push(Line::from(Span::styled(
                format!("Epoch {}", epoch.epoch),
                Style::default().fg(ACCENT),
            )));
            for (i, id) in epoch.nominees.iter().enumerate() {
                nominee_lines.push(Line::from(format!("  {}. {}", i + 1, short_id(id))));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(nominee_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" epoch N−2 nominees "),
        ),
        chunks[2],
    );
}

fn draw_logs(frame: &mut Frame, area: Rect, logs: &[String]) {
    let height = area.height.saturating_sub(2) as usize;
    let start = logs.len().saturating_sub(height);
    let lines: Vec<Line> = logs[start..]
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" logs ")),
        area,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" q ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("quit  ", Style::default().fg(MUTED)),
        Span::styled(" tab/←→ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("tabs  ", Style::default().fg(MUTED)),
        Span::styled(" 1–4 ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("jump  ", Style::default().fg(MUTED)),
        Span::styled(" --no-tui ", Style::default().fg(ACCENT)),
        Span::styled("plain logs", Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn kv(key: &str, value: impl std::fmt::Display) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:22} "), Style::default().fg(MUTED)),
        Span::raw(value.to_string()),
    ])
}

fn short_id(id: &str) -> String {
    if id.len() <= 16 {
        id.to_string()
    } else {
        format!("{}…{}", &id[..8], &id[id.len() - 6..])
    }
}

fn short_hash(hash: &str) -> String {
    if hash.len() <= 16 {
        hash.to_string()
    } else {
        format!("{}…{}", &hash[..8], &hash[hash.len() - 8..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_wrap_around() {
        assert_eq!(Tab::Logs.next(), Tab::Overview);
        assert_eq!(Tab::Overview.prev(), Tab::Logs);
        assert_eq!(Tab::Mining.index(), 1);
    }

    #[test]
    fn short_id_truncates() {
        let id = "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd";
        let s = short_id(id);
        assert!(s.starts_with("12D3KooW"));
        assert!(s.contains('…'));
        assert!(s.len() < id.len());
    }
}
