use serde::{Deserialize, Serialize};

/// Checkpoint mode for a network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CheckpointMode {
    /// Checkpoints are disabled
    #[default]
    None,
    /// User specifies checkpoints manually in the network config
    Manual,
    /// Checkpoints are triggered by consensus (on new validator set's second certified round)
    Consensus,
}

/// A manually specified checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCheckpoint {
    /// Block index that serves as the checkpoint
    pub block_index: u64,
    /// Optional block hash for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    /// Optional description of this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Requester-funded validation meter (numbers are per-network; not a mint).
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
    #[serde(default)]
    pub block_subsidy: u64,
    #[serde(default)]
    pub halving_interval_blocks: u64,
    #[serde(default)]
    pub cap: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genesis_allocations: Vec<GenesisAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GenesisAllocation {
    pub account: String,
    pub amount: u64,
}

/// Represents information about a Modality network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Name of the network (e.g., "testnet", "mainnet")
    pub name: String,

    /// Description of the network
    pub description: String,

    /// List of bootstrapper multiaddresses
    pub bootstrappers: Vec<String>,

    /// Optional static set of validators (peer IDs)
    /// If present, this network uses a static validator set.
    /// If absent, validators are selected dynamically from mining epochs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validators: Option<Vec<String>>,

    /// Checkpoint mode for this network
    /// Defaults to None if not specified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_mode: Option<CheckpointMode>,

    /// Manual checkpoints (only used when checkpoint_mode is Manual)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoints: Option<Vec<ManualCheckpoint>>,

    /// Bootstrap named **contract validators** (peer IDs). Distinct from
    /// `validators` (the sequencer committee). Empty / omitted = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_validators: Option<Vec<String>>,

    /// Minimum locked MOD to validate. Testnet/dev default is 0.
    #[serde(default)]
    pub validator_min_stake: u64,

    /// Nominal + metered validation fee schedule. Default zeros (no debit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_fees: Option<ValidationFees>,

    /// When true, dest REPOST and dest RECV apply require a prefix_cert QC.
    #[serde(default)]
    pub repost_requires_validator_cert: bool,

    /// QC threshold numerator. Default 2 (with denominator 3 → ⌈2n/3⌉).
    #[serde(default = "default_qc_numerator")]
    pub validator_qc_numerator: u64,

    /// QC threshold denominator. Default 3.
    #[serde(default = "default_qc_denominator")]
    pub validator_qc_denominator: u64,

    /// Native MOD mint schedule for this network. Omitted = no emission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emission: Option<EmissionConfig>,

    /// Blocks per mining epoch. Joiners must share this or they fork.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks_per_epoch: Option<u64>,

    /// Initial PoW difficulty. Joiners must share this or they fork.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_difficulty: Option<u64>,

    /// Target seconds per miner block (informational; applied when present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_block_time_secs: Option<u64>,
}

const DEFAULT_QC_NUMERATOR: u64 = 2;
const DEFAULT_QC_DENOMINATOR: u64 = 3;

fn default_qc_numerator() -> u64 {
    DEFAULT_QC_NUMERATOR
}

fn default_qc_denominator() -> u64 {
    DEFAULT_QC_DENOMINATOR
}

impl NetworkInfo {
    /// Get the effective checkpoint mode (defaults to None)
    pub fn get_checkpoint_mode(&self) -> CheckpointMode {
        self.checkpoint_mode.clone().unwrap_or_default()
    }

    /// Check if checkpoints are enabled
    pub fn checkpoints_enabled(&self) -> bool {
        self.get_checkpoint_mode() != CheckpointMode::None
    }

    /// Get manual checkpoints sorted by block index
    pub fn get_manual_checkpoints(&self) -> Vec<&ManualCheckpoint> {
        let mut checkpoints: Vec<_> = self
            .checkpoints
            .as_ref()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        checkpoints.sort_by_key(|c| c.block_index);
        checkpoints
    }
}

/// All available networks
pub mod networks {
    use super::NetworkInfo;

    pub fn devnet1() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet1/info.json"))
            .expect("Failed to parse devnet1 info")
    }

    pub fn devnet2() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet2/info.json"))
            .expect("Failed to parse devnet2 info")
    }

    pub fn devnet3() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet3/info.json"))
            .expect("Failed to parse devnet3 info")
    }

    pub fn devnet5() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet5/info.json"))
            .expect("Failed to parse devnet5 info")
    }

    pub fn devnet1_hybrid() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet1-hybrid/info.json"))
            .expect("Failed to parse devnet1-hybrid info")
    }

    pub fn devnet3_hybrid() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/devnet3-hybrid/info.json"))
            .expect("Failed to parse devnet3-hybrid info")
    }

    pub fn testnet() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/testnet/info.json"))
            .expect("Failed to parse testnet info")
    }

    pub fn mainnet() -> NetworkInfo {
        serde_json::from_str(include_str!("../networks/mainnet/info.json"))
            .expect("Failed to parse mainnet info")
    }

    /// Get all networks
    pub fn all() -> Vec<NetworkInfo> {
        vec![
            devnet1(),
            devnet2(),
            devnet3(),
            devnet5(),
            devnet1_hybrid(),
            devnet3_hybrid(),
            testnet(),
            mainnet(),
        ]
    }

    /// Get a network by name
    pub fn by_name(name: &str) -> Option<NetworkInfo> {
        match name {
            "devnet1" => Some(devnet1()),
            "devnet2" => Some(devnet2()),
            "devnet3" => Some(devnet3()),
            "devnet5" => Some(devnet5()),
            "devnet1-hybrid" => Some(devnet1_hybrid()),
            "devnet3-hybrid" => Some(devnet3_hybrid()),
            "testnet" => Some(testnet()),
            "mainnet" => Some(mainnet()),
            _ => None,
        }
    }
}

/// Node templates for creating pre-configured nodes
pub mod templates {
    /// Represents a node template with passfile and config
    #[derive(Debug, Clone)]
    pub struct NodeTemplate {
        pub passfile: &'static str,
        pub config: &'static str,
    }

    /// Get a node template by path (e.g., "devnet1/node1")
    pub fn get(path: &str) -> Option<NodeTemplate> {
        match path {
            "devnet1/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet1/node1/node.modal_passfile"),
                config: include_str!("../templates/devnet1/node1/config.json"),
            }),
            "devnet2/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet2/node1/node.modal_passfile"),
                config: include_str!("../templates/devnet2/node1/config.json"),
            }),
            "devnet2/node2" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet2/node2/node.modal_passfile"),
                config: include_str!("../templates/devnet2/node2/config.json"),
            }),
            "devnet3/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node1/node.modal_passfile"),
                config: include_str!("../templates/devnet3/node1/config.json"),
            }),
            "devnet3/node2" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node2/node.modal_passfile"),
                config: include_str!("../templates/devnet3/node2/config.json"),
            }),
            "devnet3/node3" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node3/node.modal_passfile"),
                config: include_str!("../templates/devnet3/node3/config.json"),
            }),
            "testnet/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/testnet/node1/node.modal_passfile"),
                config: include_str!("../templates/testnet/node1/config.json"),
            }),
            "testnet/node2" => Some(NodeTemplate {
                passfile: include_str!("../templates/testnet/node2/node.modal_passfile"),
                config: include_str!("../templates/testnet/node2/config.json"),
            }),
            "testnet/node3" => Some(NodeTemplate {
                passfile: include_str!("../templates/testnet/node3/node.modal_passfile"),
                config: include_str!("../templates/testnet/node3/config.json"),
            }),
            "testnet/node0" => Some(NodeTemplate {
                passfile: include_str!("../templates/testnet/node0/node.modal_passfile"),
                config: include_str!("../templates/testnet/node0/config.json"),
            }),
            _ => None,
        }
    }

    /// List all available templates
    pub fn list() -> Vec<&'static str> {
        vec![
            "devnet1/node1",
            "devnet2/node1",
            "devnet2/node2",
            "devnet3/node1",
            "devnet3/node2",
            "devnet3/node3",
            "testnet/node1",
            "testnet/node2",
            "testnet/node3",
            "testnet/node0",
        ]
    }
}

pub mod dns;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devnet_networks_have_validators() {
        // Test that devnet networks have validators configured
        let devnet1 = networks::devnet1();
        assert!(
            devnet1.validators.is_some(),
            "devnet1 should have validators"
        );
        assert_eq!(devnet1.validators.as_ref().unwrap().len(), 1);
        assert_eq!(
            devnet1.contract_validators.as_ref().unwrap().as_slice(),
            ["12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"]
        );
        assert!(devnet1.repost_requires_validator_cert);
        assert_eq!(devnet1.emission.as_ref().unwrap().block_subsidy, 50);

        let devnet2 = networks::devnet2();
        assert!(
            devnet2.validators.is_some(),
            "devnet2 should have validators"
        );
        assert_eq!(devnet2.validators.as_ref().unwrap().len(), 2);

        let devnet3 = networks::devnet3();
        assert!(
            devnet3.validators.is_some(),
            "devnet3 should have validators"
        );
        assert_eq!(devnet3.validators.as_ref().unwrap().len(), 3);

        let devnet5 = networks::devnet5();
        assert!(
            devnet5.validators.is_some(),
            "devnet5 should have validators"
        );
        assert_eq!(devnet5.validators.as_ref().unwrap().len(), 5);
    }

    #[test]
    fn test_testnet_mainnet_no_static_validators() {
        // Test that testnet and mainnet use dynamic validator selection
        let testnet = networks::testnet();
        assert!(
            testnet.validators.is_none(),
            "testnet should not have static validators"
        );
        assert_eq!(testnet.blocks_per_epoch, Some(40));
        assert_eq!(testnet.initial_difficulty, Some(1));
        assert_eq!(testnet.emission.as_ref().unwrap().block_subsidy, 50);
        assert_eq!(
            testnet.contract_validators.as_ref().unwrap().as_slice(),
            [
                "12D3KooWE4NPREQxLkevA5Rxd61Xiue4tTkUGN22qNABD7Mw5JhM",
                "12D3KooWJpFYTRHNuPfwoj1hTf87aqB7CDJHKtVFp3RhPNB1DrRw",
                "12D3KooWLHTsoeBE1ZWBgzumeSi6hsm3o9AndFufrGx7xLTyq2dw"
            ]
        );
        assert_eq!(testnet.validator_min_stake, 0);
        assert!(testnet.repost_requires_validator_cert);
        assert_eq!(testnet.target_block_time_secs, Some(60));
        assert_eq!(testnet.bootstrappers.len(), 3);
        for addr in &testnet.bootstrappers {
            assert!(
                addr.contains("testnet.modality.network/tcp/4040/ws/p2p/12D3KooW"),
                "testnet bootstrapper should be a named 4040 dns4 multiaddr: {addr}"
            );
        }

        let mainnet = networks::mainnet();
        assert!(
            mainnet.validators.is_none(),
            "mainnet should not have static validators"
        );
    }

    #[test]
    fn test_validator_peer_ids_are_valid() {
        // Ensure validator peer IDs are non-empty strings
        let devnet3 = networks::devnet3();
        for peer_id in devnet3.validators.unwrap() {
            assert!(!peer_id.is_empty(), "Peer ID should not be empty");
            assert!(
                peer_id.starts_with("12D3"),
                "Peer ID should be valid libp2p format"
            );
        }
    }

    #[test]
    fn test_checkpoint_mode_default() {
        // Test that checkpoint mode defaults to None
        let network = NetworkInfo {
            name: "test".to_string(),
            description: "test network".to_string(),
            bootstrappers: vec![],
            validators: None,
            checkpoint_mode: None,
            checkpoints: None,
            contract_validators: None,
            validator_min_stake: 0,
            validation_fees: None,
            repost_requires_validator_cert: false,
            validator_qc_numerator: DEFAULT_QC_NUMERATOR,
            validator_qc_denominator: DEFAULT_QC_DENOMINATOR,
            emission: None,
            blocks_per_epoch: None,
            initial_difficulty: None,
            target_block_time_secs: None,
        };
        assert_eq!(network.get_checkpoint_mode(), CheckpointMode::None);
        assert!(!network.checkpoints_enabled());
    }

    #[test]
    fn test_checkpoint_mode_consensus() {
        let network = NetworkInfo {
            name: "test".to_string(),
            description: "test network".to_string(),
            bootstrappers: vec![],
            validators: None,
            checkpoint_mode: Some(CheckpointMode::Consensus),
            checkpoints: None,
            contract_validators: None,
            validator_min_stake: 0,
            validation_fees: None,
            repost_requires_validator_cert: false,
            validator_qc_numerator: DEFAULT_QC_NUMERATOR,
            validator_qc_denominator: DEFAULT_QC_DENOMINATOR,
            emission: None,
            blocks_per_epoch: None,
            initial_difficulty: None,
            target_block_time_secs: None,
        };
        assert_eq!(network.get_checkpoint_mode(), CheckpointMode::Consensus);
        assert!(network.checkpoints_enabled());
    }

    #[test]
    fn test_manual_checkpoints() {
        let network = NetworkInfo {
            name: "test".to_string(),
            description: "test network".to_string(),
            bootstrappers: vec![],
            validators: None,
            checkpoint_mode: Some(CheckpointMode::Manual),
            contract_validators: None,
            validator_min_stake: 0,
            validation_fees: None,
            repost_requires_validator_cert: false,
            validator_qc_numerator: DEFAULT_QC_NUMERATOR,
            validator_qc_denominator: DEFAULT_QC_DENOMINATOR,
            emission: None,
            blocks_per_epoch: None,
            initial_difficulty: None,
            target_block_time_secs: None,
            checkpoints: Some(vec![
                ManualCheckpoint {
                    block_index: 100,
                    block_hash: Some("hash100".to_string()),
                    description: Some("First checkpoint".to_string()),
                },
                ManualCheckpoint {
                    block_index: 50,
                    block_hash: None,
                    description: None,
                },
            ]),
        };

        let checkpoints = network.get_manual_checkpoints();
        assert_eq!(checkpoints.len(), 2);
        // Should be sorted by block_index
        assert_eq!(checkpoints[0].block_index, 50);
        assert_eq!(checkpoints[1].block_index, 100);
    }

    #[test]
    fn test_checkpoint_mode_serialization() {
        // Test that checkpoint mode serializes to lowercase
        let json = serde_json::json!({
            "name": "test",
            "description": "test",
            "bootstrappers": [],
            "checkpoint_mode": "consensus"
        });

        let network: NetworkInfo = serde_json::from_value(json).unwrap();
        assert_eq!(network.get_checkpoint_mode(), CheckpointMode::Consensus);

        // Test manual mode
        let json = serde_json::json!({
            "name": "test",
            "description": "test",
            "bootstrappers": [],
            "checkpoint_mode": "manual",
            "checkpoints": [
                { "block_index": 100 }
            ]
        });

        let network: NetworkInfo = serde_json::from_value(json).unwrap();
        assert_eq!(network.get_checkpoint_mode(), CheckpointMode::Manual);
        assert_eq!(network.checkpoints.unwrap().len(), 1);
    }

    #[test]
    fn test_contract_validator_fields_default_when_omitted() {
        let json = serde_json::json!({
            "name": "test",
            "description": "test",
            "bootstrappers": []
        });
        let network: NetworkInfo = serde_json::from_value(json).unwrap();
        assert!(network.contract_validators.is_none());
        assert_eq!(network.validator_min_stake, 0);
        assert!(network.validation_fees.is_none());
        assert!(!network.repost_requires_validator_cert);
        assert_eq!(network.validator_qc_numerator, 2);
        assert_eq!(network.validator_qc_denominator, 3);
        assert!(network.emission.is_none());
    }

    #[test]
    fn test_contract_validator_fields_round_trip() {
        let json = serde_json::json!({
            "name": "test",
            "description": "test",
            "bootstrappers": [],
            "contract_validators": ["12D3KooWtestpeer"],
            "validator_min_stake": 0,
            "validation_fees": { "nominal": 1, "meter_coefficient": 2 },
            "repost_requires_validator_cert": true,
            "emission": { "block_subsidy": 50 }
        });
        let network: NetworkInfo = serde_json::from_value(json).unwrap();
        assert_eq!(
            network.contract_validators.as_ref().unwrap().as_slice(),
            ["12D3KooWtestpeer"]
        );
        assert_eq!(network.validation_fees.as_ref().unwrap().quote(3), 7);
        assert!(network.repost_requires_validator_cert);
        assert_eq!(network.validator_qc_numerator, 2);
        assert_eq!(network.validator_qc_denominator, 3);
        assert_eq!(network.emission.as_ref().unwrap().block_subsidy, 50);
    }

    #[test]
    fn test_testnet_bootstrap_templates_exist() {
        for name in ["testnet/node1", "testnet/node2", "testnet/node3"] {
            let tmpl = templates::get(name).unwrap_or_else(|| panic!("missing template {name}"));
            let cfg: serde_json::Value = serde_json::from_str(tmpl.config).unwrap();
            assert_eq!(cfg["network_config_path"], "modality-networks://testnet");
            assert_eq!(cfg["hybrid_consensus"], true);
            assert_eq!(cfg["run_contract_validator"], true);
            assert_eq!(cfg["listeners"][0], "/ip4/0.0.0.0/tcp/4040/ws");
            assert_eq!(cfg["passfile_path"], "./node.modal_passfile");
            let id = cfg["id"].as_str().expect("template id");
            assert!(tmpl.passfile.contains(id), "passfile must match config id");
            let status = cfg["status_url"].as_str().unwrap_or("");
            assert!(
                status.contains("testnet.modality.network"),
                "template status_url should be on modality.network"
            );
        }
        assert!(templates::list().contains(&"testnet/node1"));
    }

    #[test]
    fn test_testnet_observer_template_exists() {
        let tmpl = templates::get("testnet/node0").expect("missing template testnet/node0");
        let cfg: serde_json::Value = serde_json::from_str(tmpl.config).unwrap();
        assert_eq!(cfg["network_config_path"], "modality-networks://testnet");
        assert_eq!(cfg["status_port"], 1337);
        assert_eq!(cfg["run_miner"], false);
        assert_eq!(cfg["run_validator"], false);
        assert_eq!(cfg["listeners"][0], "/ip4/0.0.0.0/tcp/4040/ws");
        let id = cfg["id"].as_str().expect("template id");
        assert!(tmpl.passfile.contains(id), "passfile must match config id");
        assert_eq!(cfg["status_url"], "https://node0.testnet.modality.network");
        assert!(templates::list().contains(&"testnet/node0"));
    }
}
