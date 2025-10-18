use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Install modality from a cargo registry using cargo install
/// Returns the path to the newly installed binary
pub async fn install_from_registry(registry_url: &str) -> Result<PathBuf> {
    log::info!("Installing modality from registry: {}", registry_url);

    let registry_url = registry_url.to_string();

    // Find cargo binary
    let cargo_path = which::which("cargo")
        .context("Failed to find cargo binary in PATH")?;

    log::info!("Using cargo at: {}", cargo_path.display());

    // Run cargo install in a blocking task
    let output = tokio::task::spawn_blocking(move || {
        Command::new(cargo_path)
            .arg("install")
            .arg("--index")
            .arg(format!("sparse+{}", registry_url))
            .arg("modality")
            .arg("--force")
            .output()
    })
    .await
    .context("Failed to spawn cargo install command")?
    .context("Failed to execute cargo install")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("cargo install stdout: {}", stdout);
        log::error!("cargo install stderr: {}", stderr);
        anyhow::bail!("cargo install failed with status: {}", output.status);
    }

    log::info!("cargo install completed successfully");

    // Find the installed binary
    let binary_path = which::which("modality")
        .context("Failed to find newly installed modality binary")?;

    log::info!("New binary located at: {}", binary_path.display());

    Ok(binary_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cargo_exists() {
        let result = which::which("cargo");
        assert!(result.is_ok(), "cargo should be in PATH");
    }
}

