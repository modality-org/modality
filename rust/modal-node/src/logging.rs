use anyhow::Result;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::{self, Write};
use log::LevelFilter;

/// Initialize logging with optional file output and configurable log level
pub fn init_logging(logs_path: Option<PathBuf>, logs_enabled: Option<bool>, log_level: Option<String>) -> Result<()> {
    // Determine if logs should be saved to file
    let save_logs = logs_enabled.unwrap_or(true); // Default to true if not specified
    
    // Determine log level
    let level_str = log_level.unwrap_or_else(|| "info".to_string());
    let level_filter = match level_str.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" | "warning" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info, // Default to info for invalid values
    };

    // Configure env_logger
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&level_str));
    builder.filter_level(level_filter);
    
    // If logs should be saved and logs_path is specified, write to both file and terminal
    if save_logs {
        if let Some(logs_dir) = logs_path {
            // Create logs directory if specified
            std::fs::create_dir_all(&logs_dir)?;
            
            let log_file_path = logs_dir.join("node.log");
            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)?;
            
            // Create a custom target that writes to both file and terminal
            builder.target(env_logger::Target::Pipe(Box::new(DualWriter::new(log_file))));
            
            builder.init();
            
            log::info!("Logging initialized. Logs will be written to both terminal and: {} (level: {})", logs_dir.display(), level_str);
        } else {
            // No logs_path specified, just use terminal
            builder.init();
            log::info!("Logging initialized. Logs will be written to terminal only (level: {})", level_str);
        }
    } else {
        // Logs disabled, just use terminal
        builder.init();
        log::info!("Logging initialized. Logs will be written to terminal only (level: {}) - file logging disabled", level_str);
    }
    
    Ok(())
}

/// A writer that writes to both a file and stdout/stderr
struct DualWriter {
    file: std::fs::File,
}

impl DualWriter {
    fn new(file: std::fs::File) -> Self {
        Self { file }
    }
}

impl Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Write to file
        self.file.write_all(buf)?;
        // Write to stdout
        io::stdout().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        io::stdout().flush()?;
        Ok(())
    }
}
