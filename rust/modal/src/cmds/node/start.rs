//! Start a node in the background.
//!
//! This command spawns a node process in the background and returns immediately.

use anyhow::{Result, Context, bail};
use clap::Parser;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::pid::read_pid_file;

#[derive(Debug, Parser)]
#[command(about = "Start a node in the background")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Node type to run: miner, observer, validator, or server (default: determined by config)
    #[clap(long, value_parser = ["miner", "observer", "validator", "server"])]
    pub node_type: Option<String>,
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

    // Check if a node is already running
    if let Ok(Some(pid)) = read_pid_file(&node_dir) {
        #[cfg(unix)]
        {
            use nix::sys::signal;
            use nix::unistd::Pid;

            let nix_pid = Pid::from_raw(pid as i32);
            if signal::kill(nix_pid, None).is_ok() {
                bail!("Node is already running with PID {}. Use 'modal node stop' first.", pid);
            }
            // Stale PID file, we can proceed
            println!("⚠️  Found stale PID file (process {} not running), will be overwritten", pid);
        }

        #[cfg(not(unix))]
        {
            bail!("Node may already be running (PID file exists). Use 'modal node stop' first.");
        }
    }

    // Determine which node type to run
    let node_type = if let Some(ref t) = opts.node_type {
        t.clone()
    } else if let Some(ref run_as) = config.run_as {
        // Use run_as from config
        run_as.clone()
    } else if config.run_miner.unwrap_or(false) {
        "miner".to_string()
    } else {
        "server".to_string()
    };

    // Get the path to the current executable
    let current_exe = std::env::current_exe()
        .context("Failed to get current executable path")?;

    // Build the command based on node type
    let run_command = match node_type.as_str() {
        "miner" => "run-miner",
        "observer" => "run-observer",
        "validator" => "run-validator",
        "server" => "run",
        _ => bail!("Unknown node type: {}", node_type),
    };

    // Build command arguments
    let mut args = vec!["node".to_string(), run_command.to_string()];

    if let Some(ref cfg) = opts.config {
        args.push("--config".to_string());
        args.push(cfg.to_string_lossy().to_string());
    }

    // Always pass the resolved directory
    args.push("--dir".to_string());
    args.push(node_dir.to_string_lossy().to_string());

    println!("Starting {} in background...", node_type);

    // Spawn the process in the background
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // Create a new process group so the child survives parent exit
        let child = unsafe {
            Command::new(&current_exe)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(|| {
                    // Create a new session to fully detach from terminal
                    nix::unistd::setsid().ok();
                    Ok(())
                })
                .spawn()
                .context("Failed to spawn background process")?
        };

        let pid = child.id();
        println!("✓ Node started with PID: {}", pid);
        println!("  Directory: {}", node_dir.display());
        println!("  Type: {}", node_type);
        println!("\nUse 'modal node stop --dir {}' to stop the node", node_dir.display());
    }

    #[cfg(not(unix))]
    {
        let child = Command::new(&current_exe)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn background process")?;

        let pid = child.id();
        println!("✓ Node started with PID: {}", pid);
        println!("  Directory: {}", node_dir.display());
        println!("  Type: {}", node_type);
        println!("\nUse 'modal node stop --dir {}' to stop the node", node_dir.display());
    }

    Ok(())
}

