use anyhow::Result;

use crate::node::Node;

/// Run a noop node that only boots up and checks for autoupgrade
/// This mode is useful for testing autoupgrade functionality without running
/// the full network node operations (mining, consensus, etc.)
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting node in noop mode");

    // Start status server and autoupgrade if configured
    node.start_status_server().await?;
    node.start_autoupgrade().await?;

    // Log periodic status messages
    let mut status_interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Every 5 minutes
    status_interval.tick().await; // Skip the first immediate tick

    loop {
        tokio::select! {
            _ = status_interval.tick() => {
                log::info!("Noop node running - autoupgrade active: {}", 
                    node.autoupgrade_config.is_some() && 
                    node.autoupgrade_config.as_ref().unwrap().enabled
                );
            }
            _ = node.wait_for_shutdown() => {
                log::info!("Noop node shutting down");
                break;
            }
        }
    }

    Ok(())
}
