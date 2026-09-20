use serde::{Deserialize, Serialize};

/// Default dest-apply QC: ⌈2n/3⌉ named prefix signatures.
pub const VALIDATOR_QC_NUMERATOR: u64 = 2;
pub const VALIDATOR_QC_DENOMINATOR: u64 = 3;

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

/// Per-network native MOD emission. Omitted or all zeros means no mint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EmissionConfig {
    /// Smallest units minted for each canonical miner block after index 0.
    #[serde(default)]
    pub block_subsidy: u64,
    /// Halve `block_subsidy` every this many blocks. `0` means never.
    #[serde(default)]
    pub halving_interval_blocks: u64,
    /// Stop minting once this many units have been emitted (including genesis
    /// allocations). `0` means no cap.
    #[serde(default)]
    pub cap: u64,
    /// One-time credits applied when the network config is first loaded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genesis_allocations: Vec<GenesisAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GenesisAllocation {
    pub account: String,
    pub amount: u64,
}

impl EmissionConfig {
    pub fn is_active(&self) -> bool {
        self.block_subsidy > 0 || !self.genesis_allocations.is_empty()
    }

    /// Subsidy for a canonical miner block. Index 0 does not mint (allocations
    /// cover genesis). Halvings use saturating right-shift.
    pub fn subsidy_at_index(&self, index: u64) -> u64 {
        if index == 0 || self.block_subsidy == 0 {
            return 0;
        }
        let halvings = if self.halving_interval_blocks == 0 {
            0
        } else {
            (index - 1) / self.halving_interval_blocks
        };
        if halvings >= 64 {
            return 0;
        }
        self.block_subsidy >> halvings
    }
}

/// Network parameters loaded from the genesis contract.
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
    #[serde(default = "default_qc_numerator")]
    pub validator_qc_numerator: u64,
    #[serde(default = "default_qc_denominator")]
    pub validator_qc_denominator: u64,
    #[serde(default)]
    pub emission: EmissionConfig,
}

fn default_qc_numerator() -> u64 {
    VALIDATOR_QC_NUMERATOR
}

fn default_qc_denominator() -> u64 {
    VALIDATOR_QC_DENOMINATOR
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
            validator_qc_numerator: VALIDATOR_QC_NUMERATOR,
            validator_qc_denominator: VALIDATOR_QC_DENOMINATOR,
            emission: EmissionConfig::default(),
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
            validator_qc_numerator: VALIDATOR_QC_NUMERATOR,
            validator_qc_denominator: VALIDATOR_QC_DENOMINATOR,
            emission: EmissionConfig::default(),
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
        assert_eq!(params.validator_qc_numerator, VALIDATOR_QC_NUMERATOR);
        assert_eq!(params.validator_qc_denominator, VALIDATOR_QC_DENOMINATOR);
        assert!(!params.emission.is_active());
    }

    #[test]
    fn subsidy_halves_on_interval_and_skips_genesis_index() {
        let emission = EmissionConfig {
            block_subsidy: 80,
            halving_interval_blocks: 10,
            cap: 0,
            genesis_allocations: Vec::new(),
        };
        assert_eq!(emission.subsidy_at_index(0), 0);
        assert_eq!(emission.subsidy_at_index(1), 80);
        assert_eq!(emission.subsidy_at_index(10), 80);
        assert_eq!(emission.subsidy_at_index(11), 40);
        assert_eq!(emission.subsidy_at_index(21), 20);
    }
}
