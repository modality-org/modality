use serde::{Deserialize, Serialize};

/// Level of detail for inspection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InspectionLevel {
    Basic,
    Full,
    Network,
    Datastore,
    Mining,
}

impl std::str::FromStr for InspectionLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "basic" => Ok(InspectionLevel::Basic),
            "full" => Ok(InspectionLevel::Full),
            "network" => Ok(InspectionLevel::Network),
            "datastore" => Ok(InspectionLevel::Datastore),
            "mining" => Ok(InspectionLevel::Mining),
            _ => Err(format!("Unknown inspection level: {}", s)),
        }
    }
}

impl std::fmt::Display for InspectionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InspectionLevel::Basic => write!(f, "basic"),
            InspectionLevel::Full => write!(f, "full"),
            InspectionLevel::Network => write!(f, "network"),
            InspectionLevel::Datastore => write!(f, "datastore"),
            InspectionLevel::Mining => write!(f, "mining"),
        }
    }
}

/// Complete node inspection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionData {
    pub peer_id: String,
    pub status: NodeStatus,
    
    // Network information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
    
    // Datastore information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datastore: Option<DatastoreInfo>,
    
    // Mining information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mining: Option<MiningInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Running,
    Offline,
}

/// Network-related inspection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub listeners: Vec<String>,
    pub connected_peers: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_peer_list: Option<Vec<String>>,
    pub bootstrappers: Vec<String>,
}

/// Datastore-related inspection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatastoreInfo {
    pub total_blocks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_range: Option<(u64, u64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_tip_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_tip_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_miners: Option<usize>,
}

/// Mining-related inspection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningInfo {
    pub is_mining: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominees: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_hashrate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_hashes: Option<u64>,
}

impl InspectionData {
    /// Create a new basic inspection data structure
    pub fn new_basic(peer_id: String, status: NodeStatus) -> Self {
        Self {
            peer_id,
            status,
            network: None,
            datastore: None,
            mining: None,
        }
    }
    
    /// Determine which fields should be populated based on level
    pub fn should_include_network(level: InspectionLevel) -> bool {
        matches!(level, InspectionLevel::Full | InspectionLevel::Network)
    }
    
    pub fn should_include_datastore(level: InspectionLevel) -> bool {
        matches!(level, InspectionLevel::Basic | InspectionLevel::Full | InspectionLevel::Datastore)
    }
    
    pub fn should_include_mining(level: InspectionLevel) -> bool {
        matches!(level, InspectionLevel::Full | InspectionLevel::Mining)
    }
    
    /// Should include detailed peer list (not just count)
    pub fn should_include_detailed_peers(level: InspectionLevel) -> bool {
        matches!(level, InspectionLevel::Full | InspectionLevel::Network)
    }
}

