use anyhow::Result;
use std::path::PathBuf;
use crate::config::Config;

/// Resolves the config file path from either a direct config path or a node directory
/// When using node_dir, it automatically looks for config.json in that directory
pub fn resolve_config_path(config: Option<PathBuf>, node_dir: Option<PathBuf>) -> Result<PathBuf> {
    match (config, node_dir) {
        (Some(config_path), None) => {
            // Direct config path provided
            Ok(config_path)
        }
        (None, Some(node_dir_path)) => {
            // Node directory provided, look for config.json
            let config_path = node_dir_path.join("config.json");
            if !config_path.exists() {
                anyhow::bail!(
                    "Config file not found at {:?}. Expected config.json in node directory {:?}",
                    config_path,
                    node_dir_path
                );
            }
            Ok(config_path)
        }
        (Some(_), Some(_)) => {
            // Both provided - this is an error
            anyhow::bail!("Cannot specify both --config and --dir. Use one or the other.");
        }
        (None, None) => {
            // Neither provided - this should not happen as commands now default to current directory
            anyhow::bail!("Must specify either --config or --dir");
        }
    }
}

/// Loads and processes a config, automatically setting storage_path when using node_dir
pub fn load_config_with_node_dir(config: Option<PathBuf>, node_dir: Option<PathBuf>) -> Result<Config> {
    let config_path = resolve_config_path(config, node_dir.clone())?;
    let mut config = Config::from_filepath(&config_path)?;
    
    // If node_dir was used, automatically set storage_path to ./storage relative to node_dir
    // and check for node.passfile in the node directory
    if let Some(node_dir_path) = node_dir {
        let storage_path = node_dir_path.join("storage");
        config.storage_path = Some(storage_path);
        
        let logs_path = node_dir_path.join("logs");
        config.logs_path = Some(logs_path);
        
        // Check if node.passfile exists in the node directory
        let node_passfile_path = node_dir_path.join("node.passfile");
        if node_passfile_path.exists() {
            config.passfile_path = Some(node_passfile_path);
        }
    }
    
    Ok(config)
}
