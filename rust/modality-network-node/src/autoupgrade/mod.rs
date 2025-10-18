pub mod registry_checker;
pub mod installer;
pub mod self_replace;

use anyhow::{Context, Result};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::config::Config;

const DEFAULT_CHECK_INTERVAL_SECS: u64 = 3600;

/// Configuration for autoupgrade
#[derive(Debug, Clone)]
pub struct AutoupgradeConfig {
    pub enabled: bool,
    pub registry_url: String,
    pub check_interval: Duration,
}

impl AutoupgradeConfig {
    pub fn from_node_config(config: &Config) -> Option<Self> {
        let enabled = config.autoupgrade_enabled.unwrap_or(false);
        
        if !enabled {
            return None;
        }

        let registry_url = config.autoupgrade_registry_url.clone()?;
        let check_interval_secs = config.autoupgrade_check_interval_secs.unwrap_or(DEFAULT_CHECK_INTERVAL_SECS);

        Some(Self {
            enabled,
            registry_url,
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
        "Autoupgrade enabled: checking registry '{}' for package 'modality' every {:?}",
        config.registry_url,
        config.check_interval
    );

    // Get the current version at startup
    let last_known_version = registry_checker::get_current_version(&config.registry_url)
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
                log::debug!("Checking for updates in registry '{}'", config.registry_url);
                
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
    let latest_version = registry_checker::get_current_version(&config.registry_url)
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
    
    // Install the new version
    let new_binary_path = installer::install_from_registry(&config.registry_url)
        .await
        .context("Failed to install new version")?;

    log::info!("New version installed at: {}", new_binary_path.display());

    // Replace and restart
    self_replace::replace_and_restart(new_binary_path)
        .await
        .context("Failed to replace binary and restart")?;

    // If we reach here, the restart didn't work
    Ok(Some(latest_version))
}

