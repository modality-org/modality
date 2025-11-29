//! Tail the logs of a running node.
//!
//! This command shows the log output from a running node process.

use anyhow::{Result, Context, bail};
use clap::Parser;
use std::path::PathBuf;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::pid::read_pid_file;

#[derive(Debug, Parser)]
#[command(about = "Tail the logs of a running node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Number of lines to show from the end of the log (default: 50)
    #[clap(long, short = 'n', default_value = "50")]
    pub lines: usize,

    /// Follow the log file (like tail -f)
    #[clap(long, short = 'f')]
    pub follow: bool,

    /// Show logs even if node is not running
    #[clap(long)]
    pub offline: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };

    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;

    // Determine the node directory
    let node_dir = if let Some(ref d) = dir {
        d.clone()
    } else if let Some(ref cfg_path) = opts.config {
        cfg_path.parent()
            .context("Cannot determine node directory from config path")?
            .to_path_buf()
    } else {
        std::env::current_dir()?
    };

    // Check if node is running (unless --offline is specified)
    if !opts.offline {
        let is_running = check_node_running(&node_dir);
        if !is_running {
            bail!("Node is not running. Use --offline to view logs anyway, or start the node with 'modal node start'");
        }
    }

    // Determine the log file path
    let logs_path = config.logs_path
        .unwrap_or_else(|| node_dir.join("logs"));
    let log_file_path = logs_path.join("node.log");

    if !log_file_path.exists() {
        bail!("Log file not found at {}. Has the node been started?", log_file_path.display());
    }

    println!("📋 Tailing logs from: {}", log_file_path.display());
    if opts.follow {
        println!("   (Press Ctrl+C to stop)\n");
    } else {
        println!();
    }

    if opts.follow {
        tail_follow(&log_file_path, opts.lines)?;
    } else {
        tail_lines(&log_file_path, opts.lines)?;
    }

    Ok(())
}

/// Check if a node is running by verifying the PID file and process
fn check_node_running(node_dir: &std::path::Path) -> bool {
    if let Ok(Some(pid)) = read_pid_file(node_dir) {
        #[cfg(unix)]
        {
            use nix::sys::signal;
            use nix::unistd::Pid;

            let nix_pid = Pid::from_raw(pid as i32);
            return signal::kill(nix_pid, None).is_ok();
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, assume running if PID file exists
            return true;
        }
    }
    false
}

/// Show the last N lines of a file
fn tail_lines(path: &std::path::Path, n: usize) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open log file: {}", path.display()))?;
    
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines()
        .filter_map(|l| l.ok())
        .collect();
    
    let start = if lines.len() > n { lines.len() - n } else { 0 };
    
    for line in &lines[start..] {
        println!("{}", line);
    }
    
    Ok(())
}

/// Follow a file like tail -f
fn tail_follow(path: &std::path::Path, initial_lines: usize) -> Result<()> {
    use std::time::Duration;
    use std::thread;

    let mut file = File::open(path)
        .with_context(|| format!("Failed to open log file: {}", path.display()))?;
    
    // First, show the last N lines
    let reader = BufReader::new(&file);
    let lines: Vec<String> = reader.lines()
        .filter_map(|l| l.ok())
        .collect();
    
    let start = if lines.len() > initial_lines { lines.len() - initial_lines } else { 0 };
    
    for line in &lines[start..] {
        println!("{}", line);
    }
    
    // Seek to end of file
    file.seek(SeekFrom::End(0))?;
    
    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).context("Failed to set Ctrl+C handler")?;
    
    // Follow the file
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, wait a bit
                thread::sleep(Duration::from_millis(100));
            }
            Ok(_) => {
                // Remove trailing newline for consistent output
                let trimmed = line.trim_end();
                println!("{}", trimmed);
                line.clear();
            }
            Err(e) => {
                eprintln!("Error reading log file: {}", e);
                break;
            }
        }
    }
    
    println!("\n✓ Stopped following logs");
    
    Ok(())
}

