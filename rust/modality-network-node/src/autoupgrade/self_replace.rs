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

    log::info!("Binary replaced successfully, restarting process...");

    // Replace the current process with the new binary (keeps it in the foreground)
    // This ensures the user can still Ctrl-C out of it
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(&current_exe)
            .args(&args)
            .exec();
        // exec() only returns if there's an error
        return Err(anyhow::anyhow!("Failed to exec new process: {}", err));
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, spawn the process and exit
        // Note: This won't keep the process in the foreground
        Command::new(&current_exe)
            .args(&args)
            .spawn()
            .context("Failed to spawn new process")?;
        
        log::info!("New process spawned, exiting current process");
        std::process::exit(0);
    }
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

