use serde::{Deserialize, Serialize};

/// Network parameters loaded from the genesis contract
/// Note: Bootstrappers are NOT included here - they are operational/networking
/// config only and should be read from the network config file, not the genesis contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkParameters {
    pub name: String,
    pub description: String,
    pub initial_difficulty: u128,
    pub target_block_time_secs: u64,
    pub blocks_per_epoch: u64,
    pub validators: Vec<String>,
}

impl NetworkParameters {
    /// Create default parameters for testing
    pub fn default_devnet() -> Self {
        Self {
            name: "devnet".to_string(),
            description: "Development network".to_string(),
            initial_difficulty: 1,
            target_block_time_secs: 60,
            blocks_per_epoch: 40,
            validators: Vec::new(),
        }
    }
}

