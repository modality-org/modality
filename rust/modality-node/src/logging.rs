use anyhow::Result;
use env_logger::WriteStyle;
use log::{Level, LevelFilter, Record};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LOG_RING_CAPACITY: usize = 256;

/// One log line for the node TUI, with type (level) and topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub topic: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: Level, target: impl Into<String>, message: impl Into<String>) -> Self {
        let target = target.into();
        let topic = topic_of(&target);
        Self {
            level,
            target,
            topic,
            message: message.into(),
        }
    }

    pub fn from_record(record: &Record<'_>) -> Self {
        Self::new(record.level(), record.target(), record.args().to_string())
    }
}

/// Short topic used in the TUI filter, derived from the rustc log target.
pub fn topic_of(target: &str) -> String {
    if target.starts_with("libp2p")
        || target.starts_with("quinn")
        || target.starts_with("rustls")
        || target.starts_with("yamux")
        || target.starts_with("multistream")
        || target.starts_with("netlink")
    {
        return "net".to_string();
    }
    let rest = match target.split_once("::") {
        Some((head, tail)) if head.starts_with("modality") => tail,
        _ => target,
    };
    let rest = rest.strip_prefix("actions::").unwrap_or(rest);
    rest.split("::").next().unwrap_or(rest).to_string()
}

/// In-memory log lines for the node TUI.
#[derive(Clone)]
pub struct LogRing {
    lines: Arc<Mutex<VecDeque<LogEntry>>>,
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

    pub fn push(&self, entry: LogEntry) {
        let Ok(mut lines) = self.lines.lock() else {
            return;
        };
        if lines.len() >= self.cap {
            lines.pop_front();
        }
        lines.push_back(entry);
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
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
            .map_err(|_| io::Error::other("log ring pending lock poisoned"))?;
        pending.push_str(&chunk);
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
            self.push(parse_formatted_line(&line));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn parse_formatted_line(line: &str) -> LogEntry {
    let Some(rest) = line.strip_prefix('[') else {
        return LogEntry::new(Level::Info, "log", line);
    };
    let Some(end) = rest.find(']') else {
        return LogEntry::new(Level::Info, "log", line);
    };
    let header = &rest[..end];
    let message = rest[end + 1..].trim_start();
    let mut parts = header.split_whitespace();
    let first = parts.next();
    let second = parts.next();
    let third = parts.next();
    match (first, second, third) {
        (Some(_ts), Some(level), Some(target)) => {
            LogEntry::new(parse_level(level), target, message)
        }
        (Some(level), Some(target), None) => LogEntry::new(parse_level(level), target, message),
        _ => LogEntry::new(Level::Info, "log", line),
    }
}

fn parse_level(level: &str) -> Level {
    match level.to_ascii_uppercase().as_str() {
        "ERROR" => Level::Error,
        "WARN" | "WARNING" => Level::Warn,
        "INFO" => Level::Info,
        "DEBUG" => Level::Debug,
        "TRACE" => Level::Trace,
        _ => Level::Info,
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

    if let Some(ring) = sink.ring.clone() {
        builder.format(move |buf, record| {
            writeln!(
                buf,
                "[{} {:<5} {}] {}",
                buf.timestamp(),
                record.level(),
                record.target(),
                record.args()
            )?;
            ring.push(LogEntry::from_record(record));
            Ok(())
        });
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
}

impl Write for FanoutWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(ref mut file) = self.file {
            file.write_all(buf)?;
        }
        if self.write_stdout {
            io::stdout().write_all(buf)?;
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
        assert_eq!(
            lines.iter().map(|e| e.message.as_str()).collect::<Vec<_>>(),
            vec!["hello", "world"]
        );
    }

    #[test]
    fn topic_of_groups_module_paths() {
        assert_eq!(
            topic_of("modality_node::actions::miner::block_producer"),
            "miner"
        );
        assert_eq!(topic_of("modality_node::bootup"), "bootup");
        assert_eq!(topic_of("modality_node::gossip::miner::block"), "gossip");
        assert_eq!(topic_of("libp2p_swarm::behaviour"), "net");
        assert_eq!(topic_of("quinn_proto::connection"), "net");
    }

    #[test]
    fn parse_env_logger_line() {
        let e = parse_formatted_line(
            "[2026-09-20T21:13:40Z INFO  modality_node::actions::miner] Chain ready",
        );
        assert_eq!(e.level, Level::Info);
        assert_eq!(e.topic, "miner");
        assert_eq!(e.message, "Chain ready");
    }
}
