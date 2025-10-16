pub mod git_checker;
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
    pub git_repo: String,
    pub git_branch: String,
    pub check_interval: Duration,
}

impl AutoupgradeConfig {
    pub fn from_node_config(config: &Config) -> Option<Self> {
        let enabled = config.autoupgrade_enabled.unwrap_or(false);
        
        if !enabled {
            return None;
        }

        let git_repo = config.autoupgrade_git_repo.clone()?;
        let git_branch = config.autoupgrade_git_branch.clone()?;
        let check_interval_secs = config.autoupgrade_check_interval_secs.unwrap_or(DEFAULT_CHECK_INTERVAL_SECS);

        Some(Self {
            enabled,
            git_repo,
            git_branch,
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
        "Autoupgrade enabled: checking {} branch '{}' every {:?}",
        config.git_repo,
        config.git_branch,
        config.check_interval
    );

    // Get the current commit hash at startup
    let last_known_commit = git_checker::get_current_commit(&config.git_repo, &config.git_branch)
        .await
        .context("Failed to get initial commit hash")?;
    
    log::info!("Current commit on branch '{}': {}", config.git_branch, last_known_commit);

    let mut interval = tokio::time::interval(config.check_interval);
    interval.tick().await; // Skip the first immediate tick

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                log::info!("Autoupgrade task shutting down");
                break;
            }
            _ = interval.tick() => {
                log::debug!("Checking for updates on branch '{}'", config.git_branch);
                
                match check_and_upgrade(&config, &last_known_commit).await {
                    Ok(Some(new_commit)) => {
                        log::info!("Upgrade initiated to commit: {}", new_commit);
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
/// Returns Some(new_commit) if an upgrade was performed, None if no upgrade needed
async fn check_and_upgrade(
    config: &AutoupgradeConfig,
    last_known_commit: &str,
) -> Result<Option<String>> {
    let latest_commit = git_checker::get_current_commit(&config.git_repo, &config.git_branch)
        .await
        .context("Failed to check for updates")?;

    if latest_commit == last_known_commit {
        return Ok(None);
    }

    log::info!(
        "New commit detected on branch '{}': {} -> {}",
        config.git_branch,
        &last_known_commit[..8],
        &latest_commit[..8]
    );

    log::info!("Starting upgrade process...");
    
    // Install the new version
    let new_binary_path = installer::install_from_git(
        &config.git_repo,
        &config.git_branch,
    )
    .await
    .context("Failed to install new version")?;

    log::info!("New version installed at: {}", new_binary_path.display());

    // Replace and restart
    self_replace::replace_and_restart(new_binary_path)
        .await
        .context("Failed to replace binary and restart")?;

    // If we reach here, the restart didn't work
    Ok(Some(latest_commit))
}

