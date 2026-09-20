//! Shared TUI keyboard and Ctrl-C handling.
//!
//! Crossterm raw mode disables ISIG, so Ctrl-C never becomes SIGINT and the
//! process looks "stuck". We re-enable ISIG, ignore Ctrl-Z suspend, and treat
//! q / Esc / Ctrl-C as quit in both the picker and the dashboard.
//!
//! Keys come from a dedicated reader thread so the picker and dashboard can
//! each take over stdin after the other has left.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::enable_raw_mode;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

/// Owns a stdin reader thread for one TUI screen. Drop it before opening another.
pub(crate) struct KeyPump {
    rx: tokio::sync::mpsc::UnboundedReceiver<KeyEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl KeyPump {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let thread = std::thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(Event::Key(key)) => {
                            if tx.send(key).is_err() {
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
        Self {
            rx,
            stop,
            thread: Some(thread),
        }
    }

    pub async fn next_key(&mut self) -> Option<KeyEvent> {
        self.rx.recv().await
    }
}

impl Drop for KeyPump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
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
