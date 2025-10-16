use anyhow::Result;
use log::{info, warn, error};
use modality_network_datastore::{NetworkDatastore, models::miner_block::MinerBlock, Model};
use std::path::PathBuf;

/// Configuration for bootup tasks
#[derive(Debug, Clone)]
pub struct BootupConfig {
    /// Whether to run bootup tasks
    pub enabled: bool,
    /// Minimum genesis timestamp - blocks created before this will be pruned
    pub minimum_genesis_timestamp: Option<u64>,
    /// Whether to prune blocks that link back to genesis before minimum_genesis_timestamp
    pub prune_old_genesis_blocks: bool,
}

impl Default for BootupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            minimum_genesis_timestamp: None,
            prune_old_genesis_blocks: false,
        }
    }
}

/// Bootup task runner
pub struct BootupRunner {
    config: BootupConfig,
    datastore_path: PathBuf,
}

impl BootupRunner {
    pub fn new(config: BootupConfig, datastore_path: PathBuf) -> Self {
        Self {
            config,
            datastore_path,
        }
    }

    /// Run all configured bootup tasks
    pub async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Bootup tasks disabled, skipping");
            return Ok(());
        }

        info!("Starting bootup tasks...");

        // Check datastore integrity and prune blocks
        self.check_and_prune_miner_blocks().await?;

        info!("Bootup tasks completed successfully");
        Ok(())
    }

    /// Check miner block integrity and prune bad/old blocks
    async fn check_and_prune_miner_blocks(&self) -> Result<()> {
        info!("Checking miner block integrity...");

        let datastore = NetworkDatastore::create_in_directory(&self.datastore_path)?;

        // Get all miner blocks
        let all_blocks = MinerBlock::find_all_blocks(&datastore).await?;
        info!("Found {} miner blocks to check", all_blocks.len());

        let mut blocks_to_prune = Vec::new();
        let mut integrity_issues = 0;

        for block in &all_blocks {
            // Check block integrity
            if !self.is_block_valid(block) {
                warn!("Found invalid block: {} (index: {})", block.hash, block.index);
                blocks_to_prune.push(block.hash.clone());
                integrity_issues += 1;
                continue;
            }

            // Check if block should be pruned based on genesis timestamp
            if self.config.prune_old_genesis_blocks {
                if let Some(min_timestamp) = self.config.minimum_genesis_timestamp {
                    if self.should_prune_genesis_block(block, min_timestamp) {
                        info!("Pruning old genesis block: {} (timestamp: {})", 
                              block.hash, block.timestamp);
                        blocks_to_prune.push(block.hash.clone());
                    }
                }
            }
        }

        let prune_count = blocks_to_prune.len();

        // Prune identified blocks
        if !blocks_to_prune.is_empty() {
            info!("Pruning {} blocks", prune_count);
            for block_hash in blocks_to_prune {
                if let Some(block) = MinerBlock::find_by_hash(&datastore, &block_hash).await? {
                    if let Err(e) = block.delete(&datastore).await {
                        error!("Failed to prune block {}: {}", block_hash, e);
                    } else {
                        info!("Successfully pruned block: {}", block_hash);
                    }
                } else {
                    warn!("Block {} not found for pruning", block_hash);
                }
            }
        }

        if integrity_issues > 0 {
            warn!("Found {} integrity issues, pruned {} blocks", integrity_issues, prune_count);
        } else {
            info!("Miner block integrity check passed - no issues found");
        }

        Ok(())
    }

    /// Check if a block is valid
    fn is_block_valid(&self, block: &MinerBlock) -> bool {
        // Basic validation checks
        if block.hash.is_empty() {
            return false;
        }

        if block.index == 0 && !block.is_canonical {
            // Genesis block should be canonical
            return false;
        }

        // Add more validation logic as needed
        true
    }

    /// Check if a genesis block should be pruned based on timestamp
    fn should_prune_genesis_block(&self, block: &MinerBlock, min_timestamp: u64) -> bool {
        // Only prune genesis blocks (index 0) that are canonical
        if block.index != 0 || !block.is_canonical {
            return false;
        }

        // Check if block was created before minimum timestamp
        block.timestamp < min_timestamp as i64
    }
}
