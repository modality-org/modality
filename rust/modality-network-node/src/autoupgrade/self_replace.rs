use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Replace the current binary with a new one and restart the process
pub async fn replace_and_restart(new_binary_path: PathBuf) -> Result<()> {
    log::info!("Replacing current binary and restarting...");

    // Get current executable path
    let current_exe = env::current_exe()
        .context("Failed to get current executable path")?;

    log::info!("Current executable: {}", current_exe.display());
    log::info!("New executable: {}", new_binary_path.display());

    // Get current command line arguments to pass to the new process
    let args: Vec<String> = env::args().skip(1).collect();
    log::info!("Restarting with args: {:?}", args);

    // Use self_replace to replace the binary
    // This will copy the new binary to the current binary's location
    self_replace::self_replace(&new_binary_path)
        .context("Failed to replace binary")?;

    log::info!("Binary replaced successfully, spawning new process...");

    // Spawn the new binary with the same arguments
    Command::new(&current_exe)
        .args(&args)
        .spawn()
        .context("Failed to spawn new process")?;

    log::info!("New process spawned, exiting current process");
    
    // Exit the current process
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_exe_exists() {
        let result = env::current_exe();
        assert!(result.is_ok(), "Should be able to get current exe path");
    }

    #[test]
    fn test_args_collection() {
        let args: Vec<String> = env::args().collect();
        assert!(!args.is_empty(), "Should have at least the program name");
    }
}

