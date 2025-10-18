use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub version: String,
    pub git_branch: String,
    pub git_commit: String,
    pub packages: Packages,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Packages {
    pub binaries: std::collections::HashMap<String, BinaryInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BinaryInfo {
    pub name: String,
    pub path: String,
    pub platform: String,
    pub arch: String,
}

/// Fetch the manifest from the package server
pub async fn fetch_manifest(base_url: &str, branch: &str) -> Result<Manifest> {
    let manifest_url = format!("{}/{}/latest/manifest.json", base_url, branch);
    
    log::debug!("Fetching manifest from: {}", manifest_url);
    
    let response = reqwest::get(&manifest_url)
        .await
        .context("Failed to fetch manifest")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch manifest: HTTP {}", response.status()));
    }
    
    let manifest: Manifest = response
        .json()
        .await
        .context("Failed to parse manifest JSON")?;
    
    Ok(manifest)
}

/// Get the current version from the package server manifest
pub async fn get_current_version(base_url: &str, branch: &str) -> Result<String> {
    let manifest = fetch_manifest(base_url, branch).await?;
    Ok(manifest.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run manually as it requires network access
    async fn test_fetch_manifest() {
        let base_url = "http://packages.modality.org";
        let branch = "testnet";
        
        let manifest = fetch_manifest(base_url, branch).await.unwrap();
        
        assert!(!manifest.version.is_empty());
        assert!(!manifest.packages.binaries.is_empty());
    }

    #[tokio::test]
    #[ignore] // Only run manually as it requires network access
    async fn test_get_current_version() {
        let base_url = "http://packages.modality.org";
        let branch = "testnet";
        
        let version = get_current_version(base_url, branch).await.unwrap();
        
        assert!(!version.is_empty());
    }
}

