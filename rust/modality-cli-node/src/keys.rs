//! Shared TUI keyboard and Ctrl-C handling.
//!
//! Crossterm raw mode disables ISIG, so Ctrl-C never becomes SIGINT and the
//! process looks "stuck". We re-enable ISIG, ignore Ctrl-Z suspend, and treat
//! q / Esc / Ctrl-C as quit in both the picker and the dashboard.

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::enable_raw_mode;
use futures::StreamExt;
use std::io;

pub(crate) fn is_quit_key(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => true,
        KeyCode::Char('q' | 'Q') => !key.modifiers.contains(KeyModifiers::CONTROL),
        KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Char('d' | 'D') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Char('\u{3}') => true,
        _ => false,
    }
}

pub(crate) fn event_stream() -> EventStream {
    EventStream::new()
}

pub(crate) async fn next_key(events: &mut EventStream) -> Option<KeyEvent> {
    loop {
        match events.next().await {
            Some(Ok(Event::Key(key))) => return Some(key),
            Some(Ok(_)) => continue,
            Some(Err(_)) | None => return None,
        }
    }
}

/// Registers SIGINT immediately, then enables raw mode while keeping Ctrl-C.
pub(crate) fn subscribe_interrupt() -> Result<Interrupt> {
    let interrupt = Interrupt::subscribe()?;
    enable_raw_mode_keep_ctrl_c()?;
    Ok(interrupt)
}

fn enable_raw_mode_keep_ctrl_c() -> io::Result<()> {
    #[cfg(unix)]
    ignore_suspend_and_quit();
    enable_raw_mode()?;
    #[cfg(unix)]
    restore_isig();
    Ok(())
}

pub(crate) struct Interrupt {
    #[cfg(unix)]
    sigint: tokio::signal::unix::Signal,
}

impl Interrupt {
    fn subscribe() -> Result<Self> {
        #[cfg(unix)]
        {
            let sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
            Ok(Self { sigint })
        }
        #[cfg(not(unix))]
        Ok(Self {})
    }

    pub async fn recv(&mut self) {
        #[cfg(unix)]
        {
            let _ = self.sigint.recv().await;
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(unix)]
fn ignore_suspend_and_quit() {
    use nix::sys::signal::{signal, SigHandler, Signal};
    unsafe {
        let _ = signal(Signal::SIGTSTP, SigHandler::SigIgn);
        let _ = signal(Signal::SIGQUIT, SigHandler::SigIgn);
    }
}

#[cfg(unix)]
fn restore_isig() {
    use nix::sys::termios::{tcgetattr, tcsetattr, LocalFlags, SetArg};
    let stdin = std::io::stdin();
    let Ok(mut termios) = tcgetattr(&stdin) else {
        return;
    };
    termios.local_flags.insert(LocalFlags::ISIG);
    let _ = tcsetattr(&stdin, SetArg::TCSANOW, &termios);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn quit_keys() {
        assert!(is_quit_key(&key(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(is_quit_key(&key(KeyCode::Char('q'), KeyModifiers::NONE)));
        assert!(is_quit_key(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        assert!(is_quit_key(&key(
            KeyCode::Char('\u{3}'),
            KeyModifiers::NONE
        )));
        assert!(!is_quit_key(&key(KeyCode::Tab, KeyModifiers::NONE)));
        assert!(!is_quit_key(&key(KeyCode::Char('c'), KeyModifiers::NONE)));
    }
}
