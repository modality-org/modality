use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
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

/// RAII guard for PID file management.
///
/// Creates a PID file when constructed and automatically removes it when dropped.
/// This ensures the PID file is cleaned up even if the program panics.
///
/// # Example
/// ```ignore
/// let _pid_guard = PidGuard::new(&node_dir)?;
/// // PID file is automatically removed when _pid_guard goes out of scope
/// ```
pub struct PidGuard {
    node_dir: PathBuf,
    enabled: bool,
}

impl PidGuard {
    /// Create a new PID guard, writing the PID file immediately.
    pub fn new(node_dir: &Path) -> Result<Self> {
        write_pid_file(node_dir)?;
        Ok(Self {
            node_dir: node_dir.to_path_buf(),
            enabled: true,
        })
    }

    /// Create a PID guard that may or may not write a PID file.
    /// 
    /// If `node_dir` is None, no PID file is written and cleanup is skipped.
    pub fn new_optional(node_dir: Option<&Path>) -> Result<Self> {
        match node_dir {
            Some(dir) => Self::new(dir),
            None => Ok(Self {
                node_dir: PathBuf::new(),
                enabled: false,
            }),
        }
    }

    /// Manually remove the PID file and disable automatic cleanup.
    pub fn remove(mut self) -> Result<()> {
        if self.enabled {
            self.enabled = false;
            remove_pid_file(&self.node_dir)
        } else {
            Ok(())
        }
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        if self.enabled {
            if let Err(e) = remove_pid_file(&self.node_dir) {
                log::warn!("Failed to remove PID file on drop: {}", e);
            }
        }
    }
}

