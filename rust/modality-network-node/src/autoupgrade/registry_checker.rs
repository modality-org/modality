use anyhow::{Context, Result};
use std::process::Command;

/// Get the current version of the modality package from a cargo registry
pub async fn get_current_version(registry_url: &str) -> Result<String> {
    // Use tokio::task::spawn_blocking to run the blocking command
    let registry_url = registry_url.to_string();
    
    let output = tokio::task::spawn_blocking(move || {
        Command::new("cargo")
            .arg("search")
            .arg("--index")
            .arg(format!("sparse+{}", registry_url))
            .arg("modality")
            .arg("--limit")
            .arg("1")
            .output()
    })
    .await
    .context("Failed to spawn cargo search command")?
    .context("Failed to execute cargo search")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo search failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)
        .context("cargo search output is not valid UTF-8")?;

    // Parse the output: "modality = \"version\""
    let version = stdout
        .lines()
        .find(|line| line.starts_with("modality"))
        .and_then(|line| {
            line.split('=')
                .nth(1)
                .and_then(|version_part| {
                    version_part
                        .trim()
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .map(|v| v.to_string())
                })
        })
        .context("Failed to parse version from cargo search output")?;

    if version.is_empty() {
        anyhow::bail!("Empty version received from cargo search");
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run manually as it requires network access
    async fn test_get_current_version() {
        let registry_url = "http://packages.modality.org/testnet/latest/cargo-registry/index/";
        
        let version = get_current_version(registry_url).await.unwrap();
        
        // Version should be in semver format (e.g., "0.1.4")
        assert!(!version.is_empty());
        assert!(version.chars().any(|c| c.is_ascii_digit()));
    }
}
