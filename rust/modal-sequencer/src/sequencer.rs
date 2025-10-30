use crate::error::{Result, SequencerError};
use modal_observer::{ChainObserver, ForkConfig};
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for the sequencer
#[derive(Debug, Clone)]
pub struct SequencerConfig {
    /// Optional forced fork configuration for chain observation
    pub fork_config: Option<ForkConfig>,
}

impl Default for SequencerConfig {
    fn default() -> Self {
        Self {
            fork_config: None,
        }
    }
}

/// Sequencer for observing the blockchain without mining
/// 
/// The sequencer uses modal-observer to track the canonical chain
/// by observing mining events via gossip.
pub struct Sequencer {
    config: SequencerConfig,
    observer: ChainObserver,
}

impl Sequencer {
    /// Create a new sequencer with the given datastore and config
    pub async fn new(
        datastore: Arc<Mutex<NetworkDatastore>>,
        config: SequencerConfig,
    ) -> Result<Self> {
        let observer = if let Some(fork_config) = config.fork_config.clone() {
            ChainObserver::new_with_fork_config(datastore, fork_config)
        } else {
            ChainObserver::new(datastore)
        };
        
        Ok(Self { config, observer })
    }
    
    /// Create a new sequencer with default configuration
    pub async fn new_default(datastore: Arc<Mutex<NetworkDatastore>>) -> Result<Self> {
        Self::new(datastore, SequencerConfig::default()).await
    }
    
    /// Initialize the sequencer by loading existing chain state
    pub async fn initialize(&self) -> Result<()> {
        self.observer.initialize()
            .await
            .map_err(|e| SequencerError::InitializationFailed(e.to_string()))?;
        
        log::info!("Sequencer initialized successfully");
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
    async fn test_sequencer_config_default() {
        let config = SequencerConfig::default();
        assert!(config.fork_config.is_none());
    }
}

