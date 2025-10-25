use serde::{Deserialize, Serialize};

/// Represents information about a Modality network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Name of the network (e.g., "testnet", "mainnet")
    pub name: String,
    
    /// Description of the network
    pub description: String,
    
    /// List of bootstrapper multiaddresses
    pub bootstrappers: Vec<String>,
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
            "testnet" => Some(testnet()),
            "mainnet" => Some(mainnet()),
            _ => None,
        }
    }
}

pub mod dns;

