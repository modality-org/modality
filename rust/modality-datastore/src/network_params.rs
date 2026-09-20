use serde::{Deserialize, Serialize};

/// Per-network validation fee schedule (nominal + metered compute).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ValidationFees {
    #[serde(default)]
    pub nominal: u64,
    #[serde(default)]
    pub meter_coefficient: u64,
}

impl ValidationFees {
    pub fn quote(&self, gas_used: u64) -> u64 {
        self.nominal
            .saturating_add(self.meter_coefficient.saturating_mul(gas_used))
    }
}

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
    #[serde(default)]
    pub contract_validators: Vec<String>,
    #[serde(default)]
    pub validator_min_stake: u64,
    #[serde(default)]
    pub validation_fees: ValidationFees,
    #[serde(default)]
    pub repost_requires_validator_cert: bool,
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
            contract_validators: Vec::new(),
            validator_min_stake: 0,
            validation_fees: ValidationFees::default(),
            repost_requires_validator_cert: false,
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
            contract_validators: Vec::new(),
            validator_min_stake: 0,
            validation_fees: ValidationFees::default(),
            repost_requires_validator_cert: false,
        };

        assert_eq!(params.miner_hash_func, "randomx");
        assert!(params.mining_hash_params.is_some());
    }

    #[test]
    fn test_network_parameters_serde_defaults_for_contract_validators() {
        let json = serde_json::json!({
            "name": "t",
            "description": "d",
            "initial_difficulty": 1,
            "target_block_time_secs": 1,
            "blocks_per_epoch": 1,
            "validators": [],
            "miner_hash_func": "sha256"
        });
        let params: NetworkParameters = serde_json::from_value(json).unwrap();
        assert!(params.contract_validators.is_empty());
        assert!(!params.repost_requires_validator_cert);
        assert_eq!(params.validation_fees.quote(10), 0);
    }
}
