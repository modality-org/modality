//! Action picker shown by `modality node` when no subcommand is given.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::stdout;

use super::keys::{event_stream, is_quit_key, next_key, subscribe_interrupt};
use super::runner::NodeRole;
use super::tui::{install_panic_hook, TerminalGuard, ACCENT, GREEN, MUTED};

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
    let mut interrupt = subscribe_interrupt()?;
    let _guard = TerminalGuard;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let mut events = event_stream();
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
        tokio::select! {
            key = next_key(&mut events) => {
                let Some(key) = key else { break None; };
                if is_quit_key(&key) {
                    break None;
                }
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
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
                    KeyCode::Enter => {
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
            }
            _ = interrupt.recv() => break None,
        }
    };

    Ok(picked)
}

fn draw(frame: &mut Frame, menu: &ActionMenu, state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(frame.area());

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
        Span::styled(&menu.dir_display, Style::default()),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
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
                Style::default()
            } else {
                Style::default().fg(MUTED)
            };
            let title_style = if item.enabled {
                Style::default().add_modifier(Modifier::BOLD)
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
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
        .highlight_symbol(" › ");
    frame.render_stateful_widget(list, area, state);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled(
                " Enter ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("run   ", Style::default()),
            Span::styled(
                " q  Esc  Ctrl-C ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("quit", Style::default()),
        ]),
        Line::from(vec![
            Span::styled(
                " ↑↓ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("move   ", Style::default().fg(MUTED)),
            Span::styled(
                " 1-9 ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("jump (Tab does nothing here)", Style::default().fg(MUTED)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    #[test]
    fn digit_index_maps_to_items() {
        assert_eq!('1'.to_digit(10).unwrap(), 1);
        assert_eq!('9'.to_digit(10).unwrap(), 9);
    }
}
