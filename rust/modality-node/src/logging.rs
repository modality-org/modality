use anyhow::Result;
use env_logger::WriteStyle;
use log::LevelFilter;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LOG_RING_CAPACITY: usize = 256;

/// In-memory log lines for the node TUI.
#[derive(Clone)]
pub struct LogRing {
    lines: Arc<Mutex<VecDeque<String>>>,
    pending: Arc<Mutex<String>>,
    cap: usize,
}

impl LogRing {
    pub fn new() -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_RING_CAPACITY))),
            pending: Arc::new(Mutex::new(String::new())),
            cap: LOG_RING_CAPACITY,
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines
            .lock()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for LogRing {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for LogRing {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let chunk = String::from_utf8_lossy(buf);
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "log ring pending lock poisoned"))?;
        pending.push_str(&chunk);
        let mut lines = self
            .lines
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "log ring lock poisoned"))?;
        while let Some(idx) = pending.find('\n') {
            let mut line: String = pending.drain(..=idx).collect();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            if line.is_empty() {
                continue;
            }
            if lines.len() >= self.cap {
                lines.pop_front();
            }
            lines.push_back(line);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// How log lines are delivered after `env_logger` formats them.
#[derive(Clone)]
pub struct LoggingSink {
    pub write_stdout: bool,
    pub ring: Option<LogRing>,
}

impl Default for LoggingSink {
    fn default() -> Self {
        Self {
            write_stdout: true,
            ring: None,
        }
    }
}

/// Initialize logging with optional file output and configurable log level
pub fn init_logging(
    logs_path: Option<PathBuf>,
    logs_enabled: Option<bool>,
    log_level: Option<String>,
) -> Result<()> {
    init_logging_with_sink(logs_path, logs_enabled, log_level, LoggingSink::default())
}

/// Initialize logging for the terminal UI: file + in-memory ring, no stdout.
pub fn init_logging_for_tui(
    logs_path: Option<PathBuf>,
    logs_enabled: Option<bool>,
    log_level: Option<String>,
    ring: LogRing,
) -> Result<()> {
    init_logging_with_sink(
        logs_path,
        Some(logs_enabled.unwrap_or(true)),
        log_level,
        LoggingSink {
            write_stdout: false,
            ring: Some(ring),
        },
    )
}

fn init_logging_with_sink(
    logs_path: Option<PathBuf>,
    logs_enabled: Option<bool>,
    log_level: Option<String>,
    sink: LoggingSink,
) -> Result<()> {
    let save_logs = logs_enabled.unwrap_or(true);
    let level_str = log_level.unwrap_or_else(|| "info".to_string());
    let level_filter = match level_str.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" | "warning" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&level_str));
    builder.filter_level(level_filter);
    if !sink.write_stdout {
        builder.write_style(WriteStyle::Never);
    }

    let file = if save_logs {
        if let Some(logs_dir) = logs_path.clone() {
            std::fs::create_dir_all(&logs_dir)?;
            let log_file_path = logs_dir.join("node.log");
            Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path)?,
            )
        } else {
            None
        }
    } else {
        None
    };

    let writer = FanoutWriter {
        file,
        write_stdout: sink.write_stdout,
        ring: sink.ring,
    };
    builder.target(env_logger::Target::Pipe(Box::new(writer)));
    if builder.try_init().is_err() {
        return Ok(());
    }

    if let Some(logs_dir) = logs_path {
        if save_logs {
            log::info!(
                "Logging initialized (level: {}, dir: {})",
                level_str,
                logs_dir.display()
            );
        }
    } else {
        log::info!("Logging initialized (level: {})", level_str);
    }

    Ok(())
}

struct FanoutWriter {
    file: Option<std::fs::File>,
    write_stdout: bool,
    ring: Option<LogRing>,
}

impl Write for FanoutWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(ref mut file) = self.file {
            file.write_all(buf)?;
        }
        if self.write_stdout {
            io::stdout().write_all(buf)?;
        }
        if let Some(ref mut ring) = self.ring {
            ring.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            file.flush()?;
        }
        if self.write_stdout {
            io::stdout().flush()?;
        }
        if let Some(ref mut ring) = self.ring {
            ring.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_ring_splits_lines() {
        let mut ring = LogRing::new();
        ring.write_all(b"hello\nworld\n").unwrap();
        let lines = ring.snapshot();
        assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    }
}
