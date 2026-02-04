use crate::error::{Result, ValidatorError};
use modal_observer::{ChainObserver, ForkConfig};
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for the validator
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ValidatorConfig {
    /// Optional forced fork configuration for chain observation
    pub fork_config: Option<ForkConfig>,
}


/// Validator for observing the blockchain without mining
/// 
/// The validator uses modal-observer to track the canonical chain
/// by observing mining events via gossip.
pub struct Validator {
    #[allow(dead_code)]
    config: ValidatorConfig,
    observer: ChainObserver,
}

impl Validator {
    /// Create a new validator with the given datastore and config
    pub async fn new(
        datastore: Arc<Mutex<DatastoreManager>>,
        config: ValidatorConfig,
    ) -> Result<Self> {
        let observer = if let Some(fork_config) = config.fork_config.clone() {
            ChainObserver::new_with_fork_config(datastore, fork_config)
        } else {
            ChainObserver::new(datastore)
        };
        
        Ok(Self { config, observer })
    }
    
    /// Create a new validator with default configuration
    pub async fn new_default(datastore: Arc<Mutex<DatastoreManager>>) -> Result<Self> {
        Self::new(datastore, ValidatorConfig::default()).await
    }
    
    /// Initialize the validator by loading existing chain state
    pub async fn initialize(&self) -> Result<()> {
        self.observer.initialize()
            .await
            .map_err(|e| ValidatorError::InitializationFailed(e.to_string()))?;
        
        log::info!("Validator initialized successfully");
        Ok(())
    }
    
    /// Get the current chain tip from the observer
    pub async fn get_chain_tip(&self) -> u64 {
        self.observer.get_chain_tip().await
    }
    
    /// Get a reference to the chain observer
    pub fn observer(&self) -> &ChainObserver {
        &self.observer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        assert!(config.fork_config.is_none());
    }
}

