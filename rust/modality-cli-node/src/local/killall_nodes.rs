use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[cfg(target_family = "unix")]
use nix::sys::signal::kill;
#[cfg(target_family = "unix")]
use nix::unistd::Pid;

#[derive(Debug, Parser)]
#[command(about = "Kill all running modal node processes")]
pub struct Opts {
    /// Force kill (SIGKILL) instead of graceful shutdown (SIGTERM)
    #[clap(long, short)]
    pub force: bool,

    /// Dry run - show what would be killed without actually killing
    #[clap(long)]
    pub dry_run: bool,

    /// Filter by network config path (supports wildcards, e.g., "devnet*", "testnet")
    #[clap(long)]
    pub network: Option<String>,

    /// Shorthand for --network "devnet*"
    #[clap(long)]
    pub devnet: bool,

    /// Filter by directory - only kill nodes in this directory or its subdirectories (recursively)
    #[clap(long)]
    pub dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Reuse the node discovery from the nodes command
    let mut nodes = super::nodes::discover_running_nodes()?;
    
    // Apply directory filter if specified
    if let Some(dir) = &opts.dir {
        nodes = filter_nodes_by_directory(nodes, dir)?;
    }
    
    // Apply network filter if specified
    let filter = if opts.devnet {
        Some("devnet*".to_string())
    } else {
        opts.network.clone()
    };
    
    if let Some(filter) = &filter {
        nodes = super::nodes::filter_nodes_by_network(nodes, filter);
    }
    
    if nodes.is_empty() {
        if opts.dir.is_some() {
            println!("No running modal nodes found in the specified directory.");
        } else if filter.is_some() {
            println!("No running modal nodes found matching network filter.");
        } else {
            println!("No running modal nodes found.");
        }
        return Ok(());
    }
    
    println!("Found {} running node(s)", nodes.len());
    println!();
    
    if opts.dry_run {
        println!("DRY RUN - would kill the following nodes:");
        println!();
        for node in &nodes {
            println!("  PID {}: {}", node.pid, node.dir.display());
        }
        return Ok(());
    }
    
    let signal_name = if opts.force { "SIGKILL" } else { "SIGTERM" };
    println!("Killing all nodes with {}...", signal_name);
    println!();
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for node in nodes {
        print!("Killing PID {} ({})... ", node.pid, node.dir.display());
        
        #[cfg(target_family = "unix")]
        {
            let signal = if opts.force {
                nix::sys::signal::Signal::SIGKILL
            } else {
                nix::sys::signal::Signal::SIGTERM
            };
            
            match kill(Pid::from_raw(node.pid as i32), signal) {
                Ok(_) => {
                    println!("✓");
                    success_count += 1;
                    
                    // Clean up PID file if it exists
                    let pid_file = node.dir.join("node.pid");
                    if pid_file.exists() {
                        let _ = fs::remove_file(&pid_file);
                    }
                }
                Err(e) => {
                    if e == nix::errno::Errno::ESRCH {
                        println!("⚠️  Process not running (stale)");
                        // Clean up stale PID file
                        let pid_file = node.dir.join("node.pid");
                        if pid_file.exists() {
                            let _ = fs::remove_file(&pid_file);
                        }
                    } else {
                        println!("✗ Error: {}", e);
                        error_count += 1;
                    }
                }
            }
        }
        
        #[cfg(not(target_family = "unix"))]
        {
            // On non-Unix systems, use a basic approach
            let result = if opts.force {
                std::process::Command::new("taskkill")
                    .args(&["/F", "/PID", &node.pid.to_string()])
                    .output()
            } else {
                std::process::Command::new("taskkill")
                    .args(&["/PID", &node.pid.to_string()])
                    .output()
            };
            
            match result {
                Ok(output) if output.status.success() => {
                    println!("✓");
                    success_count += 1;
                    
                    // Clean up PID file if it exists
                    let pid_file = node.dir.join("node.pid");
                    if pid_file.exists() {
                        let _ = fs::remove_file(&pid_file);
                    }
                }
                Ok(_) => {
                    println!("✗ Failed to kill process");
                    error_count += 1;
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                    error_count += 1;
                }
            }
        }
    }
    
    println!();
    println!("Summary:");
    println!("  Killed: {}", success_count);
    if error_count > 0 {
        println!("  Errors: {}", error_count);
    }
    
    Ok(())
}

/// Filter nodes to only those within the specified directory or its subdirectories
fn filter_nodes_by_directory(nodes: Vec<super::nodes::NodeInfo>, dir: &PathBuf) -> Result<Vec<super::nodes::NodeInfo>> {
    // Canonicalize the directory path to handle relative paths and symlinks
    let canonical_dir = fs::canonicalize(dir)?;
    
    let filtered = nodes.into_iter()
        .filter(|node| {
            // Try to canonicalize the node's directory
            if let Ok(canonical_node_dir) = fs::canonicalize(&node.dir) {
                // Check if the node's directory starts with the specified directory
                canonical_node_dir.starts_with(&canonical_dir)
            } else {
                false
            }
        })
        .collect();
    
    Ok(filtered)
}

