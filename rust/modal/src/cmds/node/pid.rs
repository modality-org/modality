use anyhow::{Result, Context, bail};
use clap::Parser;
use std::path::PathBuf;

use modal_node::config_resolution::load_config_with_node_dir;

#[derive(Debug, Parser)]
#[command(about = "Display the PID of a running node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,
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
        bail!("No PID file found. Node may not be running.");
    }
    
    // Read PID from file
    let pid = modal_node::pid::read_pid_file(&node_dir)?
        .context("Failed to read PID from file")?;
    
    // Check if process is actually running
    #[cfg(unix)]
    {
        use nix::sys::signal;
        use nix::unistd::Pid;
        
        let nix_pid = Pid::from_raw(pid as i32);
        
        match signal::kill(nix_pid, None) {
            Ok(_) => {
                // Process exists and is running
                println!("{}", pid);
                Ok(())
            }
            Err(_) => {
                bail!("PID file exists but process {} is not running (stale PID file)", pid);
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        // On non-Unix systems, just output the PID without checking
        println!("{}", pid);
        Ok(())
    }
}

