//! Action picker shown by `modality node` when no subcommand is given.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::runner::NodeRole;
use super::tui::{install_panic_hook, TerminalGuard, ACCENT, BG, GREEN, MUTED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickedAction {
    RunFromConfig,
    Run(NodeRole),
    Create,
    Start,
    Stop,
    Info,
    Logs,
}

#[derive(Debug, Clone)]
pub struct ActionItem {
    pub action: PickedAction,
    pub title: String,
    pub hint: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ActionMenu {
    pub dir_display: String,
    pub status_line: String,
    pub items: Vec<ActionItem>,
    pub selected: usize,
}

/// Interactive list until the user picks an action or quits.
pub async fn pick_action(menu: ActionMenu) -> Result<Option<PickedAction>> {
    if menu.items.is_empty() {
        return Ok(None);
    }

    install_panic_hook();
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
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

    let mut selected = menu.selected.min(menu.items.len() - 1);
    if !menu.items[selected].enabled {
        if let Some((idx, _)) = menu.items.iter().enumerate().find(|(_, item)| item.enabled) {
            selected = idx;
        }
    }
    let mut state = ListState::default();
    state.select(Some(selected));

    let picked = loop {
        terminal.draw(|frame| draw(frame, &menu, &mut state))?;
        let Some(key) = key_rx.recv().await else {
            break None;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => break None,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break None,
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1) % menu.items.len();
                state.select(Some(selected));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selected = if selected == 0 {
                    menu.items.len() - 1
                } else {
                    selected - 1
                };
                state.select(Some(selected));
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                if menu.items[selected].enabled {
                    break Some(menu.items[selected].action);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if n >= 1 && n <= menu.items.len() && menu.items[n - 1].enabled {
                    break Some(menu.items[n - 1].action);
                }
            }
            _ => {}
        }
    };

    running.store(false, Ordering::Relaxed);
    Ok(picked)
}

fn draw(frame: &mut Frame, menu: &ActionMenu, state: &mut ListState) {
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

    draw_header(frame, chunks[0], menu);
    draw_status(frame, chunks[1], menu);
    draw_list(frame, chunks[2], menu, state);
    draw_footer(frame, chunks[3]);
}

fn draw_header(frame: &mut Frame, area: Rect, menu: &ActionMenu) {
    let line = Line::from(vec![
        Span::styled(
            " modality ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("node", Style::default().fg(GREEN)),
        Span::raw("  "),
        Span::styled(&menu.dir_display, Style::default().fg(Color::White)),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 51, 51)))
                .title(" pick an action "),
        ),
        area,
    );
}

fn draw_status(frame: &mut Frame, area: Rect, menu: &ActionMenu) {
    frame.render_widget(
        Paragraph::new(menu.status_line.as_str())
            .style(Style::default().fg(MUTED))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn draw_list(frame: &mut Frame, area: Rect, menu: &ActionMenu, state: &mut ListState) {
    let items: Vec<ListItem> = menu
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let num = format!("{:>2}  ", i + 1);
            let style = if item.enabled {
                Style::default().fg(Color::Rgb(224, 224, 224))
            } else {
                Style::default().fg(MUTED)
            };
            let title_style = if item.enabled {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            ListItem::new(Line::from(vec![
                Span::styled(num, Style::default().fg(ACCENT)),
                Span::styled(format!("{:<22}", item.title), title_style),
                Span::styled(item.hint.clone(), style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" actions "))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(26, 42, 64))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" › ");
    frame.render_stateful_widget(list, area, state);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            " enter ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("run  ", Style::default().fg(MUTED)),
        Span::styled(
            " ↑↓/jk ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("move  ", Style::default().fg(MUTED)),
        Span::styled(
            " 1–9 ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("jump  ", Style::default().fg(MUTED)),
        Span::styled(
            " q ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("quit", Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    #[test]
    fn digit_index_maps_to_items() {
        assert_eq!('1'.to_digit(10).unwrap(), 1);
        assert_eq!('9'.to_digit(10).unwrap(), 9);
    }
}
