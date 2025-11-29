//! Restart a running node.
//!
//! This command stops a running node and starts it again.

use anyhow::{Result, Context, bail};
use clap::Parser;
use std::path::PathBuf;
use std::fs;
use std::process::{Command, Stdio};

use modal_node::config_resolution::load_config_with_node_dir;

#[derive(Debug, Parser)]
#[command(about = "Restart a running node")]
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

    // Look for PID file in node directory
    let pid_file = node_dir.join("node.pid");

    // Stop the node if it's running
    if pid_file.exists() {
        // Read PID from file
        let pid_str = fs::read_to_string(&pid_file)
            .context("Failed to read PID file")?;
        let pid: i32 = pid_str.trim().parse()
            .context("Invalid PID in PID file")?;

        println!("Stopping node (PID: {})...", pid);

        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;

            let nix_pid = Pid::from_raw(pid);

            // Check if process exists
            match signal::kill(nix_pid, None) {
                Ok(_) => {
                    // Process exists, stop it
                    let signal_to_send = if opts.force {
                        Signal::SIGKILL
                    } else {
                        Signal::SIGTERM
                    };

                    signal::kill(nix_pid, signal_to_send)
                        .context("Failed to send signal to process")?;

                    // Wait for graceful shutdown
                    if !opts.force {
                        std::thread::sleep(std::time::Duration::from_secs(2));

                        // Check if process is still running
                        if signal::kill(nix_pid, None).is_ok() {
                            println!("Process still running, sending SIGKILL...");
                            signal::kill(nix_pid, Signal::SIGKILL)
                                .context("Failed to force kill process")?;
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }

                    println!("✓ Node stopped");
                }
                Err(_) => {
                    println!("⚠️  Process {} is not running (stale PID file)", pid);
                }
            }
        }

        #[cfg(not(unix))]
        {
            bail!("Restart command is only supported on Unix systems");
        }

        // Remove PID file
        fs::remove_file(&pid_file)
            .context("Failed to remove PID file")?;
    } else {
        println!("No running node found, starting fresh...");
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
        println!("✓ Node restarted with PID: {}", pid);
        println!("  Directory: {}", node_dir.display());
        println!("  Type: {}", node_type);
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
        println!("✓ Node restarted with PID: {}", pid);
        println!("  Directory: {}", node_dir.display());
        println!("  Type: {}", node_type);
    }

    Ok(())
}

