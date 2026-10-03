use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

use super::binary_checker::fetch_manifest;

/// Detect the current platform
fn detect_platform() -> Result<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let platform = match (os, arch) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => return Err(anyhow!("Unsupported platform: {} {}", os, arch)),
    };

    Ok(platform.to_string())
}

/// Download a binary from the given URL and save it to the destination path
async fn download_binary(url: &str, dest_path: &PathBuf) -> Result<()> {
    log::info!("Downloading binary from: {}", url);

    let response = reqwest::get(url)
        .await
        .context("Failed to download binary")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download binary: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read binary data")?;

    // Write to file
    fs::write(dest_path, &bytes).context("Failed to write binary file")?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(dest_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(dest_path, perms)?;
    }

    log::info!("Binary downloaded successfully to: {}", dest_path.display());

    Ok(())
}

/// Download the modality binary from the package server
/// Returns the path to the newly downloaded binary
pub async fn download_from_binary_server(base_url: &str, branch: &str) -> Result<PathBuf> {
    log::info!("Downloading modality binary from: {}/{}", base_url, branch);

    // Detect platform
    let platform = detect_platform()?;
    log::info!("Platform detected: {}", platform);

    // Fetch manifest
    let manifest = fetch_manifest(base_url, branch)
        .await
        .context("Failed to fetch manifest")?;

    log::info!("Latest version: {}", manifest.version);

    // Get binary info for this platform
    let binary_info = manifest
        .packages
        .binaries
        .get(&platform)
        .ok_or_else(|| anyhow!("No binary available for platform: {}", platform))?;

    // The versioned path: `latest` can move between our reads.
    let binary_url = format!(
        "{}/{}/{}/{}",
        base_url, branch, manifest.version, binary_info.path
    );

    // Create temporary directory for download
    let temp_dir = env::temp_dir();
    let temp_binary_path = temp_dir.join("modality_upgrade_temp");

    // Download binary
    download_binary(&binary_url, &temp_binary_path)
        .await
        .context("Failed to download binary")?;

    // Run nothing the release contract has not accepted.
    if let Err(e) = verify_release(
        base_url,
        branch,
        &manifest.version,
        &binary_info.path,
        &temp_binary_path,
    )
    .await
    {
        let _ = std::fs::remove_file(&temp_binary_path);
        return Err(e.context(format!(
            "refusing to upgrade to {}: the release contract does not accept it",
            manifest.version
        )));
    }

    log::info!("Binary downloaded to: {}", temp_binary_path.display());

    Ok(temp_binary_path)
}

/// Check a downloaded binary against the channel's release contract, pinned
/// in this build: the release `version` must be accepted, and the file's
/// sha256 must be the one it names.
pub async fn verify_release(
    base_url: &str,
    channel: &str,
    version: &str,
    package_path: &str,
    binary: &std::path::Path,
) -> Result<()> {
    use modality_common::release;
    let pin = release::pin_for(channel)
        .ok_or_else(|| anyhow!("this build pins no release contract for {channel}"))?;
    let log_url = release::log_url(base_url, channel);
    let response = reqwest::get(&log_url)
        .await
        .context("fetching the release log")?;
    if !response.status().is_success() {
        anyhow::bail!("{log_url}: HTTP {}", response.status());
    }
    let log = response.text().await?;
    let bytes = std::fs::read(binary)?;
    let entry = release::verify_download(&log, pin, channel, Some(version), package_path, &bytes)?;
    log::info!(
        "Release {} {} ({}) is accepted by release contract {}",
        channel,
        entry.version,
        entry.git_commit,
        pin.contract_id
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let result = detect_platform();
        assert!(result.is_ok(), "Should be able to detect platform");
        let platform = result.unwrap();
        assert!(!platform.is_empty());
    }

    #[tokio::test]
    #[ignore] // Only run manually as it requires network access
    async fn test_download_from_binary_server() {
        let base_url = "https://get.modality.org";
        let branch = "testnet";

        let binary_path = download_from_binary_server(base_url, branch).await.unwrap();

        assert!(binary_path.exists());

        // Clean up
        let _ = fs::remove_file(binary_path);
    }
}
