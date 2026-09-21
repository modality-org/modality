use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[cfg(target_family = "unix")]
use nix::sys::signal::kill;
#[cfg(target_family = "unix")]
use nix::unistd::Pid;

#[derive(Debug, Parser)]
#[command(about = "Find all running modal node processes")]
pub struct Opts {
    /// Show verbose output with full paths
    #[clap(long, short)]
    pub verbose: bool,

    /// Filter by network config path (supports wildcards, e.g., "devnet*", "testnet")
    #[clap(long)]
    pub network: Option<String>,

    /// Shorthand for --network "devnet*"
    #[clap(long)]
    pub devnet: bool,

    /// Filter by directory - only show nodes in this directory or its subdirectories (recursively)
    #[clap(long)]
    pub dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct NodeInfo {
    pub pid: u32,
    pub dir: PathBuf,
    pub peer_id: Option<String>,
    pub listeners: Option<Vec<String>>,
    pub network_config: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let mut nodes = discover_running_nodes()?;

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
        nodes = filter_nodes_by_network(nodes, filter);
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

    // Print header
    println!("Running Modal Nodes:");
    println!("{}", "=".repeat(80));
    println!();

    let node_count = nodes.len();
    for node in nodes {
        print_node_info(&node, opts.verbose)?;
        println!();
    }

    println!("Found {} running node(s)", node_count);

    Ok(())
}

pub fn discover_running_nodes() -> Result<Vec<NodeInfo>> {
    find_running_nodes()
}

pub fn filter_nodes_by_network(nodes: Vec<NodeInfo>, filter: &str) -> Vec<NodeInfo> {
    nodes
        .into_iter()
        .filter(|node| {
            if let Some(network_config) = &node.network_config {
                matches_network_filter(network_config, filter)
            } else {
                false
            }
        })
        .collect()
}

fn matches_network_filter(network_config: &str, filter: &str) -> bool {
    // Extract the network name from the path
    // e.g., "modality-networks://devnet3" -> "devnet3"
    let network_name = network_config
        .strip_prefix("modality-networks://")
        .unwrap_or(network_config);

    // Support simple wildcard matching
    if filter.ends_with('*') {
        let prefix = filter.trim_end_matches('*');
        network_name.starts_with(prefix)
    } else {
        network_name == filter
    }
}

fn find_running_nodes() -> Result<Vec<NodeInfo>> {
    let mut nodes = Vec::new();

    #[cfg(target_family = "unix")]
    {
        // Find all modal processes
        let output = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("modal.*run")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let pids_str = String::from_utf8_lossy(&output.stdout);

                for pid_str in pids_str.lines() {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        if let Some(node_info) = get_node_info_from_pid(pid) {
                            nodes.push(node_info);
                        }
                    }
                }
            }
        }

        // Also check for PID files under the current tree
        let search_roots = vec![
            std::env::current_dir().ok(),
            Some(PathBuf::from(".")),
            Some(PathBuf::from("./tmp")),
            Some(PathBuf::from("../tmp")),
            Some(PathBuf::from("../../tmp")),
        ];

        let mut seen_pids = std::collections::HashSet::new();
        for node in &nodes {
            seen_pids.insert(node.pid);
        }

        for base_path in search_roots.into_iter().flatten() {
            let mut dirs = Vec::new();
            collect_node_dirs(&base_path, 5, &mut dirs);
            for path in dirs {
                let pid_file = path.join("node.pid");
                if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        if is_process_running(pid) && !seen_pids.contains(&pid) {
                            if let Some(node_info) = get_node_info_from_dir(&path, pid) {
                                seen_pids.insert(pid);
                                nodes.push(node_info);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_family = "unix"))]
    {
        // On non-Unix systems, just look for PID files
        // This is a simplified version - could be enhanced with Windows-specific tools
        println!("Note: Process discovery limited on non-Unix systems");
    }

    Ok(nodes)
}

#[cfg(target_family = "unix")]
fn is_process_running(pid: u32) -> bool {
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(target_family = "unix"))]
fn is_process_running(_pid: u32) -> bool {
    true // Assume running on non-Unix
}

#[cfg(target_family = "unix")]
fn get_node_info_from_pid(pid: u32) -> Option<NodeInfo> {
    let cwd_link = format!("/proc/{}/cwd", pid);

    if let Ok(cwd) = fs::read_link(&cwd_link) {
        if let Some(info) = get_node_info_from_dir(&cwd, pid) {
            return Some(info);
        }
    }

    if let Some(cwd) = macos_cwd(pid) {
        if let Some(info) = get_node_info_from_dir(&cwd, pid) {
            return Some(info);
        }
    }

    if let Some(dir) = dir_from_ps_args(pid) {
        if let Some(info) = get_node_info_from_dir(&dir, pid) {
            return Some(info);
        }
    }

    // Last resort: lsof paths whose basename is node.pid or config.json
    let output = std::process::Command::new("lsof")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-Fn")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            for line in output_str.lines() {
                if let Some(path_str) = line.strip_prefix('n') {
                    let path = PathBuf::from(path_str);

                    if path.file_name().and_then(|n| n.to_str()) == Some("node.pid") {
                        if let Some(dir) = path.parent() {
                            if let Some(info) = get_node_info_from_dir(dir, pid) {
                                return Some(info);
                            }
                        }
                    }

                    if path.file_name().and_then(|n| n.to_str()) == Some("config.json") {
                        if let Some(dir) = path.parent() {
                            if dir.join("node.pid").exists()
                                || dir.join("node.modal_passfile").exists()
                            {
                                if let Some(info) = get_node_info_from_dir(dir, pid) {
                                    return Some(info);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_family = "unix")]
fn macos_cwd(pid: u32) -> Option<PathBuf> {
    let output = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
        if let Some(path_str) = line.strip_prefix('n') {
            let path = PathBuf::from(path_str);
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(target_family = "unix")]
fn dir_from_ps_args(pid: u32) -> Option<PathBuf> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-ww", "-o", "args="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let args = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = args.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "--dir" {
            if let Some(dir) = parts.get(i + 1) {
                return Some(PathBuf::from(dir));
            }
        }
        if let Some(dir) = part.strip_prefix("--dir=") {
            return Some(PathBuf::from(dir));
        }
    }
    None
}

fn collect_node_dirs(dir: &std::path::Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    if dir.join("node.pid").exists() {
        out.push(dir.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            name,
            "target" | ".git" | "node_modules" | "data" | "storage" | "logs"
        ) {
            continue;
        }
        collect_node_dirs(&path, depth.saturating_sub(1), out);
    }
}

#[cfg(not(target_family = "unix"))]
fn get_node_info_from_pid(_pid: u32) -> Option<NodeInfo> {
    None
}

fn get_node_info_from_dir(dir: &std::path::Path, pid: u32) -> Option<NodeInfo> {
    let config_path = dir.join("config.json");

    if !config_path.exists() {
        return None;
    }

    // Try to read the config
    let config_result = fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok());

    let (peer_id, listeners, network_config) = if let Some(config) = config_result {
        let peer_id = config
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let listeners = config
            .get("listeners")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        // Extract network_config_path
        let network_config = config
            .get("network_config_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        (peer_id, listeners, network_config)
    } else {
        (None, None, None)
    };

    Some(NodeInfo {
        pid,
        dir: dir.to_path_buf(),
        peer_id,
        listeners,
        network_config,
    })
}

fn print_node_info(node: &NodeInfo, verbose: bool) -> Result<()> {
    println!("PID: {}", node.pid);

    if verbose {
        println!("Directory: {}", node.dir.display());
    } else {
        // Try to show a shorter, more readable path
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(rel_path) = node.dir.strip_prefix(&current_dir) {
                println!("Directory: ./{}", rel_path.display());
            } else {
                println!("Directory: {}", node.dir.display());
            }
        } else {
            println!("Directory: {}", node.dir.display());
        }
    }

    if let Some(peer_id) = &node.peer_id {
        println!("Peer ID: {}", peer_id);
    }

    if let Some(network_config) = &node.network_config {
        println!("Network: {}", network_config);
    }

    if let Some(listeners) = &node.listeners {
        if !listeners.is_empty() {
            println!("Listening addresses:");
            for listener in listeners {
                // Append peer ID to make complete multiaddr
                if let Some(peer_id) = &node.peer_id {
                    println!("  • {}/p2p/{}", listener, peer_id);
                } else {
                    println!("  • {}", listener);
                }
            }
        }
    }

    Ok(())
}

/// Filter nodes to only those within the specified directory or its subdirectories
pub fn filter_nodes_by_directory(nodes: Vec<NodeInfo>, dir: &PathBuf) -> Result<Vec<NodeInfo>> {
    // Canonicalize the directory path to handle relative paths and symlinks
    let canonical_dir = fs::canonicalize(dir)?;

    let filtered = nodes
        .into_iter()
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
