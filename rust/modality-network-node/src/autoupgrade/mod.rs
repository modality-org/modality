pub mod binary_checker;
pub mod installer;
pub mod self_replace;

use anyhow::{Context, Result};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::config::Config;

const DEFAULT_CHECK_INTERVAL_SECS: u64 = 3600;
const DEFAULT_BASE_URL: &str = "http://get.modal.money";
const DEFAULT_BRANCH: &str = "testnet";

/// Configuration for autoupgrade
#[derive(Debug, Clone)]
pub struct AutoupgradeConfig {
    pub enabled: bool,
    pub base_url: String,
    pub branch: String,
    pub check_interval: Duration,
}

impl AutoupgradeConfig {
    pub fn from_node_config(config: &Config) -> Option<Self> {
        let enabled = config.autoupgrade_enabled.unwrap_or(false);
        
        if !enabled {
            return None;
        }

        let base_url = config.autoupgrade_base_url.clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let branch = config.autoupgrade_branch.clone()
            .unwrap_or_else(|| DEFAULT_BRANCH.to_string());
        let check_interval_secs = config.autoupgrade_check_interval_secs.unwrap_or(DEFAULT_CHECK_INTERVAL_SECS);

        Some(Self {
            enabled,
            base_url,
            branch,
            check_interval: Duration::from_secs(check_interval_secs),
        })
    }
}

/// Start the autoupgrade background task
pub async fn start_autoupgrade_task(
    config: AutoupgradeConfig,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!(
        "Autoupgrade enabled: checking {}/{} for package 'modality' every {:?}",
        config.base_url,
        config.branch,
        config.check_interval
    );

    // Get the current version at startup
    let last_known_version = binary_checker::get_current_version(&config.base_url, &config.branch)
        .await
        .context("Failed to get initial version")?;
    
    log::info!("Current version of 'modality': {}", last_known_version);

    let mut interval = tokio::time::interval(config.check_interval);
    interval.tick().await; // Skip the first immediate tick

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                log::info!("Autoupgrade task shutting down");
                break;
            }
            _ = interval.tick() => {
                log::debug!("Checking for updates at {}/{}", config.base_url, config.branch);
                
                match check_and_upgrade(&config, &last_known_version).await {
                    Ok(Some(new_version)) => {
                        log::info!("Upgrade initiated to version: {}", new_version);
                        // The upgrade process will replace this binary and restart
                        // If we reach here, something went wrong
                        return Err(anyhow::anyhow!("Upgrade process completed but node still running"));
                    }
                    Ok(None) => {
                        log::debug!("No updates available");
                    }
                    Err(e) => {
                        log::error!("Error checking for updates: {}", e);
                        // Continue checking despite errors
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check for updates and upgrade if available
/// Returns Some(new_version) if an upgrade was performed, None if no upgrade needed
async fn check_and_upgrade(
    config: &AutoupgradeConfig,
    last_known_version: &str,
) -> Result<Option<String>> {
    let latest_version = binary_checker::get_current_version(&config.base_url, &config.branch)
        .await
        .context("Failed to check for updates")?;

    if latest_version == last_known_version {
        return Ok(None);
    }

    log::info!(
        "New version detected: {} -> {}",
        last_known_version,
        latest_version
    );

    log::info!("Starting upgrade process...");
    
    // Download the new binary
    let new_binary_path = installer::download_from_binary_server(&config.base_url, &config.branch)
        .await
        .context("Failed to download new version")?;

    log::info!("New version downloaded to: {}", new_binary_path.display());

    // Replace and restart
    self_replace::replace_and_restart(new_binary_path)
        .await
        .context("Failed to replace binary and restart")?;

    // If we reach here, the restart didn't work
    Ok(Some(latest_version))
}

