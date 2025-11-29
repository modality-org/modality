//! Directory resolution utilities for node commands.

use anyhow::Result;
use std::path::PathBuf;

/// Resolves the node directory from config and dir options.
///
/// Logic:
/// - If neither config nor dir is provided, defaults to current directory
/// - If dir is provided, uses that directory
/// - If only config is provided, uses current directory
///
/// # Arguments
/// * `config` - Optional path to a configuration file
/// * `dir` - Optional path to the node directory
///
/// # Returns
/// The resolved directory path, or an error if the current directory cannot be determined.
pub fn resolve_node_dir(config: &Option<PathBuf>, dir: &Option<PathBuf>) -> Result<Option<PathBuf>> {
    if config.is_none() && dir.is_none() {
        Ok(Some(std::env::current_dir()?))
    } else {
        Ok(dir.clone())
    }
}

/// Resolves the node directory, returning a concrete PathBuf (never None).
///
/// This is useful when you always need a directory path, even if just for PID file management.
#[allow(dead_code)]
pub fn resolve_node_dir_required(config: &Option<PathBuf>, dir: &Option<PathBuf>) -> Result<PathBuf> {
    match resolve_node_dir(config, dir)? {
        Some(d) => Ok(d),
        None => std::env::current_dir().map_err(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_node_dir_both_none() {
        let result = resolve_node_dir(&None, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_resolve_node_dir_with_dir() {
        let dir = Some(PathBuf::from("/tmp/test"));
        let result = resolve_node_dir(&None, &dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_resolve_node_dir_with_config_only() {
        let config = Some(PathBuf::from("/tmp/config.json"));
        let result = resolve_node_dir(&config, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}

