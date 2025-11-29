use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use modal_common::hash_tax;

/// Special peer ID used for the genesis block (no nomination)
pub const GENESIS_PEER_ID: &str = "";

/// Fixed timestamp for default genesis block (Unix epoch: Jan 1, 1970 00:00:00 UTC)
/// This ensures all nodes create identical genesis blocks
pub const GENESIS_TIMESTAMP: i64 = 0;

/// Block data containing a nominated peer ID and arbitrary number
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockData {
    /// Peer ID nominated by the miner (to be used downstream)
    pub nominated_peer_id: String,
    /// Arbitrary number selected by the miner
    pub miner_number: u64,
}

impl BlockData {
    pub fn new(nominated_peer_id: String, miner_number: u64) -> Self {
        Self {
            nominated_peer_id,
            miner_number,
        }
    }
    
    /// Serialize block data to JSON-compatible string for hashing
    pub fn to_hash_string(&self) -> String {
        format!("{}{}", self.nominated_peer_id, self.miner_number)
    }
}


/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub index: u64,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: String,
    pub data_hash: String,  // Hash of the BlockData
    pub nonce: u128,
    pub difficulty: u128,
    pub hash: String,
}

impl BlockHeader {
    /// Create block data string for mining (excludes hash and nonce initially)
    pub fn mining_data(&self) -> String {
        format!(
            "{}{}{}{}{}",
            self.index,
            self.timestamp.timestamp(),
            self.previous_hash,
            self.data_hash,
            self.difficulty
        )
    }
    
    /// Calculate hash of header with given nonce
    pub fn calculate_hash(&self, nonce: u128) -> String {
        // Use RandomX for hashing - no fallback
        hash_tax::hash_with_nonce(&self.mining_data(), nonce, "randomx")
            .expect("RandomX hashing failed - this should not happen in production")
    }
}

/// A block in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub data: BlockData,
}

impl Block {
    /// Create a new block (before mining)
    pub fn new(
        index: u64,
        previous_hash: String,
        data: BlockData,
        difficulty: u128,
    ) -> Self {
        let data_hash = Self::calculate_data_hash(&data);
        
        let header = BlockHeader {
            index,
            timestamp: Utc::now(),
            previous_hash,
            data_hash,
            nonce: 0,
            difficulty,
            hash: String::new(),
        };
        
        Self {
            header,
            data,
        }
    }
    
    /// Create the default genesis block (shared by all nodes)
    /// 
    /// The default genesis block:
    /// - Has index 0 with no nomination (empty nominated_peer_id)
    /// - Uses a fixed timestamp (Unix epoch) for deterministic hash
    /// - Precedes epoch 0 (not counted as part of any epoch)
    /// 
    /// All nodes using default genesis will produce identical genesis blocks.
    pub fn default_genesis(difficulty: u128) -> Self {
        let data = BlockData::new(GENESIS_PEER_ID.to_string(), 0);
        let data_hash = Self::calculate_data_hash(&data);
        
        let header = BlockHeader {
            index: 0,
            timestamp: Utc.timestamp_opt(GENESIS_TIMESTAMP, 0).unwrap(),
            previous_hash: "0".to_string(),
            data_hash,
            nonce: 0,
            difficulty,
            hash: String::new(),
        };
        
        let mut block = Self { header, data };
        
        // Genesis block doesn't need mining, just set hash
        block.header.hash = block.header.calculate_hash(0);
        block
    }
    
    /// Create genesis block with a specific peer ID (for custom networks)
    /// 
    /// This is useful for networks that want a specific node to be credited
    /// in the genesis block. For shared/public networks, use `default_genesis()`.
    #[deprecated(note = "Use default_genesis() for shared networks")]
    pub fn genesis(difficulty: u128, genesis_peer_id: String) -> Self {
        let data = BlockData::new(genesis_peer_id, 0);
        
        let mut block = Self::new(
            0,
            "0".to_string(),
            data,
            difficulty,
        );
        
        // Genesis block doesn't need mining, just set hash
        block.header.hash = block.header.calculate_hash(0);
        block
    }
    
    /// Check if this is the genesis block (block 0)
    pub fn is_genesis(&self) -> bool {
        self.header.index == 0 && self.header.previous_hash == "0"
    }
    
    /// Calculate hash of block data
    fn calculate_data_hash(data: &BlockData) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.to_hash_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Verify the data hash is correct
    pub fn verify_data_hash(&self) -> bool {
        let calculated = Self::calculate_data_hash(&self.data);
        calculated == self.header.data_hash
    }
    
    /// Verify this block's hash is valid
    pub fn verify_hash(&self) -> bool {
        let calculated = self.header.calculate_hash(self.header.nonce);
        calculated == self.header.hash
    }
    
    /// Get the mining data for this block
    pub fn mining_data(&self) -> String {
        self.header.mining_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_data_creation() {
        let data = BlockData::new("peer_id_123".to_string(), 12345);

        assert_eq!(data.miner_number, 12345);
        assert_eq!(data.nominated_peer_id, "peer_id_123");
        assert!(!data.to_hash_string().is_empty());
    }

    #[test]
    fn test_default_genesis_block() {
        let genesis = Block::default_genesis(1);

        assert_eq!(genesis.header.index, 0);
        assert_eq!(genesis.header.previous_hash, "0");
        assert_eq!(genesis.data.miner_number, 0);
        assert_eq!(genesis.data.nominated_peer_id, ""); // No nomination
        assert!(!genesis.header.hash.is_empty());
        assert!(genesis.is_genesis());
    }
    
    #[test]
    fn test_default_genesis_deterministic() {
        // Two default genesis blocks should be identical
        let genesis1 = Block::default_genesis(1);
        let genesis2 = Block::default_genesis(1);
        
        assert_eq!(genesis1.header.hash, genesis2.header.hash);
        assert_eq!(genesis1.header.timestamp, genesis2.header.timestamp);
        assert_eq!(genesis1.header.data_hash, genesis2.header.data_hash);
    }
    
    #[test]
    #[allow(deprecated)]
    fn test_custom_genesis_block() {
        let genesis = Block::genesis(1, "custom_peer_id".to_string());

        assert_eq!(genesis.header.index, 0);
        assert_eq!(genesis.header.previous_hash, "0");
        assert_eq!(genesis.data.miner_number, 0);
        assert_eq!(genesis.data.nominated_peer_id, "custom_peer_id");
        assert!(!genesis.header.hash.is_empty());
    }

    #[test]
    fn test_data_hash() {
        let data = BlockData::new("peer_id_abc".to_string(), 42);

        let block = Block::new(1, "prev".to_string(), data, 1);

        assert!(block.verify_data_hash());
    }

    #[test]
    fn test_block_hash_calculation() {
        let data = BlockData::new("peer_id_test".to_string(), 100);

        let block = Block::new(1, "prev".to_string(), data, 1);

        let hash1 = block.header.calculate_hash(0);
        let hash2 = block.header.calculate_hash(0);
        let hash3 = block.header.calculate_hash(1);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}

