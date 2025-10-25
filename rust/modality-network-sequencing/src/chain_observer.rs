use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::NetworkDatastore;
use std::sync::Arc;
use tokio::sync::Mutex;

/// ChainObserver tracks the canonical mining chain without participating in mining
/// 
/// This is used by sequencer nodes that need to observe the mining chain
/// to perform consensus operations but do not mine blocks themselves.
pub struct ChainObserver {
    datastore: Arc<Mutex<NetworkDatastore>>,
    chain_tip_index: Arc<Mutex<u64>>,
}

impl ChainObserver {
    /// Create a new chain observer
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>) -> Self {
        Self {
            datastore,
            chain_tip_index: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Initialize the observer by loading the current chain tip
    pub async fn initialize(&self) -> Result<()> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        
        if let Some(max_index) = canonical_blocks.iter().map(|b| b.index).max() {
            let mut tip = self.chain_tip_index.lock().await;
            *tip = max_index;
            log::info!("Chain observer initialized at tip index: {}", max_index);
        } else {
            log::info!("Chain observer initialized with empty chain");
        }
        
        Ok(())
    }
    
    /// Get the current chain tip index
    pub async fn get_chain_tip(&self) -> u64 {
        *self.chain_tip_index.lock().await
    }
    
    /// Update the chain tip index
    /// This should be called when new blocks are received via gossip
    pub async fn update_chain_tip(&self, new_tip: u64) -> Result<()> {
        let mut tip = self.chain_tip_index.lock().await;
        if new_tip > *tip {
            log::info!("Chain tip updated from {} to {}", *tip, new_tip);
            *tip = new_tip;
        }
        Ok(())
    }
    
    /// Get the canonical block at a specific index
    pub async fn get_canonical_block(&self, index: u64) -> Result<Option<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_canonical_by_index(&ds, index).await?)
    }
    
    /// Get all canonical blocks
    pub async fn get_all_canonical_blocks(&self) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_all_canonical(&ds).await?)
    }
    
    /// Get the canonical blocks for a specific epoch
    pub async fn get_canonical_blocks_by_epoch(&self, epoch: u64) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_canonical_by_epoch(&ds, epoch).await?)
    }
    
    /// Calculate the cumulative difficulty of the current canonical chain
    pub async fn get_chain_cumulative_difficulty(&self) -> Result<u128> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        MinerBlock::calculate_cumulative_difficulty(&canonical_blocks)
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_chain_observer_creation() {
        // This is a basic test - in practice you'd need a real datastore
        // Just verifying the structure compiles and basic APIs work
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        let observer = ChainObserver::new(datastore);
        
        // Should start at 0
        assert_eq!(observer.get_chain_tip().await, 0);
    }
}

