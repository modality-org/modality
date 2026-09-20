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
    pub miner_hash_func: String,
    pub mining_hash_params: Option<serde_json::Value>,
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
            miner_hash_func: "randomx".to_string(),
            mining_hash_params: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_includes_miner_hash_func() {
        let params = NetworkParameters::default_devnet();
        assert_eq!(params.miner_hash_func, "randomx");
        assert!(params.mining_hash_params.is_none());
    }
    
    #[test]
    fn test_network_parameters_with_custom_hash_params() {
        let custom_params = serde_json::json!({
            "key": "test-key",
            "flags": "recommended"
        });
        
        let params = NetworkParameters {
            name: "testnet".to_string(),
            description: "Test network".to_string(),
            initial_difficulty: 100,
            target_block_time_secs: 30,
            blocks_per_epoch: 20,
            validators: vec!["peer1".to_string()],
            miner_hash_func: "randomx".to_string(),
            mining_hash_params: Some(custom_params),
        };
        
        assert_eq!(params.miner_hash_func, "randomx");
        assert!(params.mining_hash_params.is_some());
    }
}

