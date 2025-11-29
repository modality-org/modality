use anyhow::{Result, Context, bail};
use clap::Parser;
use std::path::PathBuf;
use std::fs;

use modal_node::config_resolution::load_config_with_node_dir;

#[derive(Debug, Parser)]
#[command(about = "Kill a running node process")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Force kill (SIGKILL) instead of graceful shutdown (SIGTERM)
    #[clap(long, short)]
    pub force: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let _config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
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
    
    // Look for PID file in node directory
    let pid_file = node_dir.join("node.pid");
    
    if !pid_file.exists() {
        bail!("No PID file found at {}. Is the node running?", pid_file.display());
    }
    
    // Read PID from file
    let pid_str = fs::read_to_string(&pid_file)
        .context("Failed to read PID file")?;
    let pid: i32 = pid_str.trim().parse()
        .context("Invalid PID in PID file")?;
    
    println!("Found node process with PID: {}", pid);
    
    // Check if process is actually running
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        
        let nix_pid = Pid::from_raw(pid);
        
        // Check if process exists
        match signal::kill(nix_pid, None) {
            Ok(_) => {
                // Process exists, kill it
                let signal_to_send = if opts.force {
                    Signal::SIGKILL
                } else {
                    Signal::SIGTERM
                };
                
                println!("Sending {} to process {}...", 
                    if opts.force { "SIGKILL" } else { "SIGTERM" }, 
                    pid);
                
                signal::kill(nix_pid, signal_to_send)
                    .context("Failed to send signal to process")?;
                
                // Wait a bit for graceful shutdown
                if !opts.force {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    
                    // Check if process is still running
                    if signal::kill(nix_pid, None).is_ok() {
                        println!("Process still running, sending SIGKILL...");
                        signal::kill(nix_pid, Signal::SIGKILL)
                            .context("Failed to force kill process")?;
                    }
                }
                
                println!("✓ Node process killed successfully");
            }
            Err(_) => {
                println!("⚠️  Process {} is not running (stale PID file)", pid);
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        bail!("Kill command is only supported on Unix systems");
    }
    
    // Remove PID file
    fs::remove_file(&pid_file)
        .context("Failed to remove PID file")?;
    println!("✓ PID file removed");
    
    Ok(())
}

