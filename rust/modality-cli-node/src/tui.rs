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
use std::collections::HashSet;
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

const TYPE_CHIPS: [Level; 5] = [
    Level::Error,
    Level::Warn,
    Level::Info,
    Level::Debug,
    Level::Trace,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChipRow {
    Type,
    Topic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChipCursor {
    row: ChipRow,
    type_index: usize,
    topic_index: usize,
}

impl Default for ChipCursor {
    fn default() -> Self {
        Self {
            row: ChipRow::Type,
            type_index: 2, // INFO, the usual noise
            topic_index: 0,
        }
    }
}

impl ChipCursor {
    fn index(&self) -> usize {
        match self.row {
            ChipRow::Type => self.type_index,
            ChipRow::Topic => self.topic_index,
        }
    }

    fn clamp(&mut self, topic_count: usize) {
        self.type_index = self.type_index.min(TYPE_CHIPS.len() - 1);
        if topic_count == 0 {
            self.row = ChipRow::Type;
            self.topic_index = 0;
        } else {
            self.topic_index = self.topic_index.min(topic_count - 1);
        }
    }

    fn left(&mut self) {
        match self.row {
            ChipRow::Type => self.type_index = self.type_index.saturating_sub(1),
            ChipRow::Topic => self.topic_index = self.topic_index.saturating_sub(1),
        }
    }

    fn right(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        match self.row {
            ChipRow::Type => {
                if self.type_index + 1 < len {
                    self.type_index += 1;
                }
            }
            ChipRow::Topic => {
                if self.topic_index + 1 < len {
                    self.topic_index += 1;
                }
            }
        }
    }

    fn up(&mut self) {
        self.row = ChipRow::Type;
    }

    fn down(&mut self, topic_count: usize) {
        if topic_count > 0 {
            self.row = ChipRow::Topic;
        }
    }
}

/// Inclusive filters: everything is on until the user turns a chip off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LogFilter {
    disabled_levels: HashSet<Level>,
    disabled_topics: HashSet<String>,
}

impl LogFilter {
    fn matches(&self, entry: &LogEntry) -> bool {
        if self.disabled_levels.contains(&entry.level) {
            return false;
        }
        if self.disabled_topics.contains(&entry.topic) {
            return false;
        }
        true
    }

    fn level_on(&self, level: Level) -> bool {
        !self.disabled_levels.contains(&level)
    }

    fn topic_on(&self, topic: &str) -> bool {
        !self.disabled_topics.contains(topic)
    }

    fn toggle_level(&mut self, level: Level) {
        if !self.disabled_levels.remove(&level) {
            self.disabled_levels.insert(level);
        }
        if self.disabled_levels.len() == TYPE_CHIPS.len() {
            self.disabled_levels.remove(&level);
        }
    }

    fn toggle_topic(&mut self, topic: &str) {
        if !self.disabled_topics.remove(topic) {
            self.disabled_topics.insert(topic.to_string());
        }
    }

    fn only_level(&mut self, level: Level) {
        self.disabled_levels = TYPE_CHIPS.iter().copied().filter(|l| *l != level).collect();
    }

    fn only_topic(&mut self, topic: &str, all_topics: &[String]) {
        self.disabled_topics = all_topics
            .iter()
            .filter(|t| t.as_str() != topic)
            .cloned()
            .collect();
    }

    fn reset(&mut self) {
        self.disabled_levels.clear();
        self.disabled_topics.clear();
    }

    fn hiding(&self) -> bool {
        !self.disabled_levels.is_empty() || !self.disabled_topics.is_empty()
    }
}

fn topic_names(logs: &[LogEntry]) -> Vec<String> {
    let mut topics: Vec<String> = logs.iter().map(|e| e.topic.clone()).collect();
    topics.sort();
    topics.dedup();
    topics
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
    let mut cursor = ChipCursor::default();
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
        let topics = topic_names(&log_lines);
        cursor.clamp(topics.len());
        terminal.draw(|frame| {
            draw(
                frame,
                snapshot.as_ref(),
                &log_lines,
                &filter,
                cursor,
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
                    KeyCode::Left | KeyCode::Char('h') => cursor.left(),
                    KeyCode::Right | KeyCode::Char('l') => {
                        let len = match cursor.row {
                            ChipRow::Type => TYPE_CHIPS.len(),
                            ChipRow::Topic => topics.len(),
                        };
                        cursor.right(len);
                    }
                    KeyCode::Up | KeyCode::Char('k') => cursor.up(),
                    KeyCode::Down | KeyCode::Char('j') => cursor.down(topics.len()),
                    KeyCode::Char(' ') => match cursor.row {
                        ChipRow::Type => {
                            filter.toggle_level(TYPE_CHIPS[cursor.index().min(TYPE_CHIPS.len() - 1)]);
                        }
                        ChipRow::Topic => {
                            if let Some(topic) = topics.get(cursor.index()) {
                                filter.toggle_topic(topic);
                            }
                        }
                    },
                    KeyCode::Enter => match cursor.row {
                        ChipRow::Type => {
                            filter.only_level(TYPE_CHIPS[cursor.index().min(TYPE_CHIPS.len() - 1)]);
                        }
                        ChipRow::Topic => {
                            if let Some(topic) = topics.get(cursor.index()) {
                                filter.only_topic(topic, &topics);
                            }
                        }
                    },
                    KeyCode::Char('0' | 'a' | 'A') => filter.reset(),
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
    cursor: ChipCursor,
    stop_node_on_leave: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], status, stop_node_on_leave);
    draw_epoch(frame, chunks[1], status);
    draw_stats(frame, chunks[2], status);
    draw_blocks(frame, chunks[3], status);
    draw_logs(frame, chunks[4], logs, filter, cursor);
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
            s.role_display.as_str(),
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
        kv("Role", &s.role_display),
        kv("Hybrid", hybrid),
        kv("Difficulty", &s.current_difficulty),
        kv(
            "Hashrate",
            format!(
                "miner {} H/s   network {} H/s",
                s.miner_hashrate, s.network_hashrate
            ),
        ),
        kv("Blocks mined here", s.blocks_mined_by_node.to_string()),
        kv(
            "Sequencing",
            format!(
                "round {}  ·  {} canonical",
                s.current_round, s.total_miner_blocks
            ),
        ),
    ];
    if let Some(url) = &s.status_url {
        lines.push(kv("Status page", url));
    }
    if !s.listeners.is_empty() {
        lines.push(kv("Listen", s.listeners.join(", ")));
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

fn draw_logs(
    frame: &mut Frame,
    area: Rect,
    logs: &[LogEntry],
    filter: &LogFilter,
    cursor: ChipCursor,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(log_title(logs, filter));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let topics = topic_names(logs);
    frame.render_widget(type_chip_line(logs, filter, cursor, rows[0].width), rows[0]);
    frame.render_widget(
        topic_chip_line(logs, filter, cursor, &topics, rows[1].width),
        rows[1],
    );

    let filtered: Vec<&LogEntry> = logs.iter().filter(|e| filter.matches(e)).collect();
    let height = rows[2].height as usize;
    let start = filtered.len().saturating_sub(height);
    let lines: Vec<Line> = if filtered.is_empty() {
        vec![Line::from(Span::styled(
            if logs.is_empty() {
                "waiting for log lines…"
            } else {
                "nothing matches  ·  space toggles  ·  enter isolates  ·  0 shows all"
            },
            Style::default().fg(MUTED),
        ))]
    } else {
        filtered[start..].iter().map(|e| log_line(e)).collect()
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[2]);
}

fn log_title(logs: &[LogEntry], filter: &LogFilter) -> String {
    let shown = logs.iter().filter(|e| filter.matches(e)).count();
    if !filter.hiding() {
        format!(" logs  {shown} ")
    } else {
        format!(" logs  {shown} of {}  ·  0 show all ", logs.len())
    }
}

struct ChipView {
    label: String,
    count: usize,
    on: bool,
    color: Color,
}

fn type_chip_line(
    logs: &[LogEntry],
    filter: &LogFilter,
    cursor: ChipCursor,
    width: u16,
) -> Paragraph<'static> {
    let chips: Vec<ChipView> = TYPE_CHIPS
        .iter()
        .map(|level| ChipView {
            label: level.to_string().to_ascii_lowercase(),
            count: logs.iter().filter(|e| e.level == *level).count(),
            on: filter.level_on(*level),
            color: level_color(*level),
        })
        .collect();
    let selected = (cursor.row == ChipRow::Type).then_some(cursor.index());
    Paragraph::new(chip_row("type", &chips, selected, width))
}

fn topic_chip_line(
    logs: &[LogEntry],
    filter: &LogFilter,
    cursor: ChipCursor,
    topics: &[String],
    width: u16,
) -> Paragraph<'static> {
    if topics.is_empty() {
        return Paragraph::new(Line::from(vec![
            Span::styled("topic ", Style::default().fg(MUTED)),
            Span::styled("waiting…", Style::default().fg(MUTED)),
        ]));
    }
    let chips: Vec<ChipView> = topics
        .iter()
        .map(|topic| ChipView {
            label: topic.clone(),
            count: logs.iter().filter(|e| e.topic == *topic).count(),
            on: filter.topic_on(topic),
            color: ACCENT,
        })
        .collect();
    let selected = (cursor.row == ChipRow::Topic).then_some(cursor.index());
    Paragraph::new(chip_row("topic", &chips, selected, width))
}

fn chip_row(prefix: &str, chips: &[ChipView], cursor: Option<usize>, width: u16) -> Line<'static> {
    let mut items: Vec<(String, Style)> = Vec::with_capacity(chips.len() + 1);
    items.push((format!("{prefix:<5} "), Style::default().fg(MUTED)));
    for (i, chip) in chips.iter().enumerate() {
        let text = if chip.count > 0 {
            format!(" {} {} ", chip.label, chip.count)
        } else {
            format!(" {} ", chip.label)
        };
        let mut style = if chip.on {
            Style::default().fg(chip.color).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(MUTED)
                .add_modifier(Modifier::CROSSED_OUT)
        };
        if cursor == Some(i) {
            style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
        }
        items.push((text, style));
    }

    let prefix_len = items[0].0.chars().count();
    let budget = (width as usize).saturating_sub(prefix_len);
    let widths: Vec<usize> = items[1..].iter().map(|(t, _)| t.chars().count()).collect();
    let cursor_i = cursor.unwrap_or(0).min(widths.len().saturating_sub(1));
    let mut start = 0;
    if !widths.is_empty() {
        let mut used: usize = widths[start..=cursor_i].iter().sum();
        while start < cursor_i && used > budget {
            used = used.saturating_sub(widths[start]);
            start += 1;
        }
    }

    let mut spans = vec![Span::styled(items[0].0.clone(), items[0].1)];
    if start > 0 {
        spans.push(Span::styled("‹", Style::default().fg(MUTED)));
    }
    let mut used = 0usize;
    let extra = if start > 0 { 1 } else { 0 };
    for (i, (text, style)) in items[1..].iter().enumerate() {
        if i < start {
            continue;
        }
        let w = text.chars().count();
        if i > start && used + extra + w > budget {
            spans.push(Span::styled("›", Style::default().fg(MUTED)));
            break;
        }
        spans.push(Span::styled(text.clone(), *style));
        used += w;
    }
    Line::from(spans)
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
    let leave = if stop_node_on_leave {
        "stop the node"
    } else {
        "back to menu · node keeps running"
    };
    let lines = vec![
        Line::from(vec![
            key_hint(" ←→ "),
            Span::styled("chip  ", Style::default().fg(MUTED)),
            key_hint(" ↑↓ "),
            Span::styled("row  ", Style::default().fg(MUTED)),
            key_hint(" space "),
            Span::styled("on/off  ", Style::default().fg(MUTED)),
            key_hint(" enter "),
            Span::styled("only this  ", Style::default().fg(MUTED)),
            key_hint(" 0 "),
            Span::styled("show all", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            key_hint(" q "),
            Span::styled(" or ", Style::default().fg(MUTED)),
            key_hint(" Esc "),
            Span::styled(" or ", Style::default().fg(MUTED)),
            key_hint(" Ctrl-C "),
            Span::styled(format!("  {leave}"), Style::default().fg(MUTED)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
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
        let logs = vec![LogEntry::new(
            Level::Info,
            "modality_node::actions::miner",
            "hello log",
        )];
        let filter = LogFilter::default();
        let cursor = ChipCursor::default();
        terminal
            .draw(|frame| draw(frame, None, &logs, &filter, cursor, false))
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
        assert!(text.contains("type"), "type chips missing:\n{text}");
        assert!(text.contains("topic"), "topic chips missing:\n{text}");
        assert!(text.contains("info"), "info type chip missing:\n{text}");
        assert!(text.contains("miner"), "log topic chip missing:\n{text}");
        assert!(text.contains("space"), "toggle hint missing:\n{text}");
        assert!(text.contains("only this"), "isolate hint missing:\n{text}");
        assert!(text.contains("show all"), "reset hint missing:\n{text}");
        assert!(
            !text.contains("l type") && !text.contains("t topic"),
            "old cycle hints still on screen:\n{text}"
        );

        terminal
            .draw(|frame| draw(frame, None, &logs, &filter, cursor, true))
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

    #[test]
    fn log_filter_toggles_type_and_topic_chips() {
        let entries = vec![
            LogEntry::new(Level::Info, "modality_node::node", "Listening on /ip4/…"),
            LogEntry::new(
                Level::Warn,
                "modality_node::actions::miner",
                "hashrate dropped",
            ),
            LogEntry::new(Level::Error, "modality_node::gossip::miner", "peer lost"),
        ];
        let mut filter = LogFilter::default();
        assert_eq!(entries.iter().filter(|e| filter.matches(e)).count(), 3);

        filter.toggle_level(Level::Info);
        let shown: Vec<_> = entries
            .iter()
            .filter(|e| filter.matches(e))
            .map(|e| e.message.as_str())
            .collect();
        assert_eq!(shown, vec!["hashrate dropped", "peer lost"]);

        filter.reset();
        filter.toggle_topic("node");
        filter.toggle_topic("gossip");
        let shown: Vec<_> = entries
            .iter()
            .filter(|e| filter.matches(e))
            .map(|e| e.topic.as_str())
            .collect();
        assert_eq!(shown, vec!["miner"]);

        filter.reset();
        filter.only_level(Level::Error);
        let shown: Vec<_> = entries
            .iter()
            .filter(|e| filter.matches(e))
            .map(|e| e.level)
            .collect();
        assert_eq!(shown, vec![Level::Error]);

        let names = topic_names(&entries);
        filter.reset();
        filter.only_topic("miner", &names);
        let shown: Vec<_> = entries
            .iter()
            .filter(|e| filter.matches(e))
            .map(|e| e.topic.as_str())
            .collect();
        assert_eq!(shown, vec!["miner"]);
    }

    #[test]
    fn chip_cursor_moves_between_rows() {
        let mut cursor = ChipCursor::default();
        assert_eq!(cursor.row, ChipRow::Type);
        cursor.down(3);
        assert_eq!(cursor.row, ChipRow::Topic);
        assert_eq!(cursor.index(), 0);
        cursor.right(3);
        assert_eq!(cursor.index(), 1);
        cursor.up();
        assert_eq!(cursor.row, ChipRow::Type);
        assert_eq!(cursor.index(), 2);
    }
}
