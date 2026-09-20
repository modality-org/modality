//! Terminal UI for a running node.
//!
//! Shown by default for foreground `node run*` when stdout is a TTY.
//! One screen: status, recent blocks, logs. Quit with q / Esc / Ctrl-C.
//! Tab is ignored (IDE terminals often steal it for pane focus).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use log::Level;
use modality_node::logging::{LogEntry, LogRing};
use modality_node::status_snapshot::{NodeStatus, NodeStatusSource};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, Terminal};
use std::io::stdout;
use std::time::Duration;

use super::keys::{is_quit_key, subscribe_interrupt, KeyPump};

/// Named ANSI colors so the dashboard follows the terminal palette
/// instead of painting a fixed RGB scheme.
pub(crate) const ACCENT: Color = Color::Cyan;
pub(crate) const GREEN: Color = Color::Green;
pub(crate) const MUTED: Color = Color::DarkGray;
const WARN: Color = Color::Yellow;
const ERROR: Color = Color::Red;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogFilter {
    /// Inclusive maximum verbosity. `None` shows every type in the ring.
    max_level: Option<Level>,
    topic: Option<String>,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            max_level: None,
            topic: None,
        }
    }
}

impl LogFilter {
    fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(max) = self.max_level {
            if entry.level > max {
                return false;
            }
        }
        if let Some(ref topic) = self.topic {
            if entry.topic != *topic {
                return false;
            }
        }
        true
    }

    fn level_label(&self) -> &'static str {
        match self.max_level {
            None => "all",
            Some(Level::Error) => "error+",
            Some(Level::Warn) => "warn+",
            Some(Level::Info) => "info+",
            Some(Level::Debug) => "debug+",
            Some(Level::Trace) => "trace+",
        }
    }

    fn topic_label(&self) -> &str {
        self.topic.as_deref().unwrap_or("all")
    }

    fn cycle_level(&mut self) {
        self.max_level = match self.max_level {
            None => Some(Level::Error),
            Some(Level::Error) => Some(Level::Warn),
            Some(Level::Warn) => Some(Level::Info),
            Some(Level::Info) => Some(Level::Debug),
            Some(Level::Debug) => Some(Level::Trace),
            Some(Level::Trace) => None,
        };
    }

    fn cycle_topic(&mut self, entries: &[LogEntry]) {
        let mut topics: Vec<String> = entries.iter().map(|e| e.topic.clone()).collect();
        topics.sort();
        topics.dedup();
        if topics.is_empty() {
            self.topic = None;
            return;
        }
        self.topic = match &self.topic {
            None => Some(topics[0].clone()),
            Some(current) => match topics.iter().position(|t| t == current) {
                Some(i) if i + 1 < topics.len() => Some(topics[i + 1].clone()),
                _ => None,
            },
        };
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

pub(crate) struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, LeaveAlternateScreen);
        let _ = execute!(out, crossterm::cursor::Show);
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
        let _ = execute!(out, crossterm::cursor::Show);
        original(info);
    }));
}

/// Run the node dashboard until the user leaves it or the node shuts down.
///
/// When `stop_node_on_leave` is true (standalone `node run*`), leaving the
/// dashboard stops the node. When false (picker session), the node keeps running.
pub async fn run(source: NodeStatusSource, logs: LogRing, stop_node_on_leave: bool) -> Result<()> {
    install_panic_hook();
    let mut interrupt = subscribe_interrupt()?;
    let _guard = TerminalGuard;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, crossterm::cursor::Hide)?;
    let _shutdown = stop_node_on_leave.then(|| ShutdownOnExit(source.clone()));
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let mut keys = KeyPump::new();
    let mut filter = LogFilter::default();
    let mut snapshot = None;
    let mut snap_task = {
        let src = source.clone();
        Some(tokio::spawn(async move { src.snapshot().await }))
    };
    let mut interval = tokio::time::interval(Duration::from_millis(750));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut shutdown_rx = source.shutdown_tx.subscribe();

    loop {
        let log_lines = logs.snapshot();
        terminal.draw(|frame| {
            draw(
                frame,
                snapshot.as_ref(),
                &log_lines,
                &filter,
                stop_node_on_leave,
            )
        })?;

        tokio::select! {
            biased;
            key = keys.next_key() => {
                let Some(key) = key else { break; };
                if is_quit_key(&key) {
                    break;
                }
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('l' | 'L') => filter.cycle_level(),
                    KeyCode::Char('t' | 'T') => filter.cycle_topic(&log_lines),
                    KeyCode::Char('0') => filter.reset(),
                    _ => {}
                }
            }
            _ = interrupt.recv() => break,
            _ = shutdown_rx.recv() => break,
            result = async {
                snap_task.as_mut().unwrap().await
            }, if snap_task.is_some() => {
                snap_task = None;
                match result {
                    Ok(Ok(s)) => snapshot = Some(s),
                    Ok(Err(e)) => log::debug!("status snapshot failed: {e}"),
                    Err(e) => log::debug!("status snapshot task join failed: {e}"),
                }
            }
            _ = interval.tick() => {
                if snap_task.is_none() {
                    let src = source.clone();
                    snap_task = Some(tokio::spawn(async move { src.snapshot().await }));
                }
            }
        }
    }

    Ok(())
}

fn draw(
    frame: &mut Frame,
    status: Option<&NodeStatus>,
    logs: &[LogEntry],
    filter: &LogFilter,
    stop_node_on_leave: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], status, stop_node_on_leave);
    draw_epoch(frame, chunks[1], status);
    draw_stats(frame, chunks[2], status);
    draw_blocks(frame, chunks[3], status);
    draw_logs(frame, chunks[4], logs, filter);
    draw_footer(frame, chunks[5], stop_node_on_leave);
}

fn draw_header(
    frame: &mut Frame,
    area: Rect,
    status: Option<&NodeStatus>,
    stop_node_on_leave: bool,
) {
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
            "starting",
            "…",
            "…".into(),
            "0".into(),
            "–".into(),
            "–".into(),
            "–".into(),
        ),
    };

    let line = Line::from(vec![
        Span::styled(
            " modality ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(role, Style::default().fg(GREEN)),
        Span::raw("  "),
        Span::styled(network, Style::default().fg(MUTED)),
        Span::raw("  peer "),
        Span::styled(peer, Style::default()),
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
                .border_style(Style::default().fg(MUTED))
                .title(if stop_node_on_leave {
                    " node · q / Esc / Ctrl-C stop "
                } else {
                    " node · q / Esc back to menu "
                }),
        ),
        area,
    );
}

fn draw_epoch(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            " epoch {}  ({}/{} blocks) ",
            s.current_epoch,
            s.chain_tip % s.blocks_per_epoch,
            s.blocks_per_epoch
        )))
        .gauge_style(Style::default().fg(ACCENT).bg(MUTED))
        .ratio(s.epoch_progress().clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

fn draw_stats(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };

    let hybrid = if s.hybrid_consensus {
        "on (N−2 sequencers)"
    } else {
        "off"
    };
    let mut lines = vec![
        kv("Role", &s.role),
        kv("Hybrid", hybrid),
        kv("Difficulty", &s.current_difficulty),
        kv(
            "Hashrate",
            &format!(
                "miner {} H/s   network {} H/s",
                s.miner_hashrate, s.network_hashrate
            ),
        ),
        kv("Blocks mined here", &s.blocks_mined_by_node.to_string()),
        kv(
            "Sequencing",
            &format!(
                "round {}  ·  {} canonical",
                s.current_round, s.total_miner_blocks
            ),
        ),
    ];
    if let Some(url) = &s.status_url {
        lines.push(kv("Status page", url));
    }
    if !s.listeners.is_empty() {
        lines.push(kv("Listen", &s.listeners.join(", ")));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title(" status ")),
        area,
    );
}

fn draw_blocks(frame: &mut Frame, area: Rect, status: Option<&NodeStatus>) {
    let Some(s) = status else {
        frame.render_widget(Paragraph::new("Collecting status…"), area);
        return;
    };

    let header = Row::new(["Index", "Epoch", "Hash", "Nominee"])
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    let rows = s.recent_blocks.iter().take(5).map(|b| {
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
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" recent blocks "),
    );
    frame.render_widget(table, area);
}

fn draw_logs(frame: &mut Frame, area: Rect, logs: &[LogEntry], filter: &LogFilter) {
    let filtered: Vec<&LogEntry> = logs.iter().filter(|e| filter.matches(e)).collect();
    let height = area.height.saturating_sub(2) as usize;
    let start = filtered.len().saturating_sub(height);
    let lines: Vec<Line> = if filtered.is_empty() {
        vec![Line::from(Span::styled(
            if logs.is_empty() {
                "waiting for log lines…"
            } else {
                "no lines for this type/topic   ·   l / t to cycle   0 to reset"
            },
            Style::default().fg(MUTED),
        ))]
    } else {
        filtered[start..]
            .iter()
            .map(|e| log_line(e))
            .collect()
    };
    let title = format!(
        " logs  {}/{}  type:{}  topic:{}  ·  l type  t topic  0 reset ",
        filtered.len(),
        logs.len(),
        filter.level_label(),
        filter.topic_label(),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn log_line(entry: &LogEntry) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<5}", entry.level),
            Style::default().fg(level_color(entry.level)),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<10}", truncate_topic(&entry.topic, 10)),
            Style::default().fg(ACCENT),
        ),
        Span::raw(" "),
        Span::raw(entry.message.clone()),
    ])
}

fn level_color(level: Level) -> Color {
    match level {
        Level::Error => ERROR,
        Level::Warn => WARN,
        Level::Info => GREEN,
        Level::Debug | Level::Trace => MUTED,
    }
}

fn truncate_topic(topic: &str, width: usize) -> String {
    if topic.len() <= width {
        topic.to_string()
    } else {
        format!("{}…", &topic[..width.saturating_sub(1)])
    }
}

fn key_hint(label: &'static str) -> Span<'static> {
    Span::styled(
        label,
        Style::default()
            .fg(ACCENT)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD),
    )
}

fn draw_footer(frame: &mut Frame, area: Rect, stop_node_on_leave: bool) {
    let action = if stop_node_on_leave {
        "  stop the node"
    } else {
        "  back to menu  ·  node keeps running"
    };
    let line = Line::from(vec![
        key_hint(" q "),
        Span::styled(" or ", Style::default().fg(MUTED)),
        key_hint(" Esc "),
        Span::styled(" or ", Style::default().fg(MUTED)),
        key_hint(" Ctrl-C "),
        Span::styled(action, Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn kv(key: &str, value: impl std::fmt::Display) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:18} "), Style::default().fg(MUTED)),
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
    use ratatui::backend::TestBackend;

    #[test]
    fn short_id_truncates() {
        let id = "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd";
        let s = short_id(id);
        assert!(s.starts_with("12D3KooW"));
        assert!(s.contains('…'));
        assert!(s.len() < id.len());
    }

    #[test]
    fn dashboard_puts_quit_on_the_screen() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| draw(frame, None, &["hello log".into()], false))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in buffer.area.top()..buffer.area.bottom() {
            for x in buffer.area.left()..buffer.area.right() {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        assert!(
            text.contains("menu") && text.contains("keeps running"),
            "back-to-menu hint missing from dashboard:\n{text}"
        );
        assert!(
            !text.to_lowercase().contains("overview") || !text.contains("Tab  ←"),
            "tab navigation should not be the main UI:\n{text}"
        );
        assert!(text.contains("hello log"));

        terminal
            .draw(|frame| draw(frame, None, &["hello log".into()], true))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut stop_text = String::new();
        for y in buffer.area.top()..buffer.area.bottom() {
            for x in buffer.area.left()..buffer.area.right() {
                stop_text.push_str(buffer[(x, y)].symbol());
            }
            stop_text.push('\n');
        }
        assert!(
            stop_text.contains("stop the node"),
            "standalone quit hint missing:\n{stop_text}"
        );

        for y in buffer.area.top()..buffer.area.bottom() {
            for x in buffer.area.left()..buffer.area.right() {
                let cell = &buffer[(x, y)];
                assert!(
                    !matches!(cell.fg, Color::Rgb(_, _, _))
                        && !matches!(cell.bg, Color::Rgb(_, _, _)),
                    "dashboard cell ({x},{y}) uses RGB instead of the terminal palette: fg={:?} bg={:?}",
                    cell.fg,
                    cell.bg
                );
            }
        }
    }
}
