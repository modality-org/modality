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
        let mut checkpoints: Vec<_> = self.checkpoints.as_ref()
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
                passfile: include_str!("../templates/devnet1/node1/node.passfile"),
                config: include_str!("../templates/devnet1/node1/config.json"),
            }),
            "devnet2/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet2/node1/node.passfile"),
                config: include_str!("../templates/devnet2/node1/config.json"),
            }),
            "devnet2/node2" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet2/node2/node.passfile"),
                config: include_str!("../templates/devnet2/node2/config.json"),
            }),
            "devnet3/node1" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node1/node.passfile"),
                config: include_str!("../templates/devnet3/node1/config.json"),
            }),
            "devnet3/node2" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node2/node.passfile"),
                config: include_str!("../templates/devnet3/node2/config.json"),
            }),
            "devnet3/node3" => Some(NodeTemplate {
                passfile: include_str!("../templates/devnet3/node3/node.passfile"),
                config: include_str!("../templates/devnet3/node3/config.json"),
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
        assert!(devnet1.validators.is_some(), "devnet1 should have validators");
        assert_eq!(devnet1.validators.as_ref().unwrap().len(), 1);

        let devnet2 = networks::devnet2();
        assert!(devnet2.validators.is_some(), "devnet2 should have validators");
        assert_eq!(devnet2.validators.as_ref().unwrap().len(), 2);

        let devnet3 = networks::devnet3();
        assert!(devnet3.validators.is_some(), "devnet3 should have validators");
        assert_eq!(devnet3.validators.as_ref().unwrap().len(), 3);

        let devnet5 = networks::devnet5();
        assert!(devnet5.validators.is_some(), "devnet5 should have validators");
        assert_eq!(devnet5.validators.as_ref().unwrap().len(), 5);
    }

    #[test]
    fn test_testnet_mainnet_no_static_validators() {
        // Test that testnet and mainnet use dynamic validator selection
        let testnet = networks::testnet();
        assert!(testnet.validators.is_none(), "testnet should not have static validators");

        let mainnet = networks::mainnet();
        assert!(mainnet.validators.is_none(), "mainnet should not have static validators");
    }

    #[test]
    fn test_validator_peer_ids_are_valid() {
        // Ensure validator peer IDs are non-empty strings
        let devnet3 = networks::devnet3();
        for peer_id in devnet3.validators.unwrap() {
            assert!(!peer_id.is_empty(), "Peer ID should not be empty");
            assert!(peer_id.starts_with("12D3"), "Peer ID should be valid libp2p format");
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
}

