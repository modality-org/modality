use anyhow::{Result, Context};
use std::path::Path;
use std::fs;

/// Write the current process ID to a PID file in the specified directory
pub fn write_pid_file(node_dir: &Path) -> Result<()> {
    let pid_file = node_dir.join("node.pid");
    let pid = std::process::id();
    
    fs::write(&pid_file, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", pid_file.display()))?;
    
    log::info!("PID file written: {} (PID: {})", pid_file.display(), pid);
    
    Ok(())
}

/// Remove the PID file from the specified directory
pub fn remove_pid_file(node_dir: &Path) -> Result<()> {
    let pid_file = node_dir.join("node.pid");
    
    if pid_file.exists() {
        fs::remove_file(&pid_file)
            .with_context(|| format!("Failed to remove PID file: {}", pid_file.display()))?;
        log::info!("PID file removed: {}", pid_file.display());
    }
    
    Ok(())
}

/// Read the PID from a PID file
pub fn read_pid_file(node_dir: &Path) -> Result<Option<u32>> {
    let pid_file = node_dir.join("node.pid");
    
    if !pid_file.exists() {
        return Ok(None);
    }
    
    let pid_str = fs::read_to_string(&pid_file)
        .with_context(|| format!("Failed to read PID file: {}", pid_file.display()))?;
    
    let pid: u32 = pid_str.trim().parse()
        .with_context(|| format!("Invalid PID in file: {}", pid_file.display()))?;
    
    Ok(Some(pid))
}

