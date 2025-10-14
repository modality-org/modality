use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ed25519_dalek::VerifyingKey;

/// Block data containing a nominated public key and arbitrary number
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockData {
    /// Ed25519 public key nominated by the miner (to be used downstream)
    #[serde(with = "public_key_serde")]
    pub nominated_public_key: VerifyingKey,
    /// Arbitrary number selected by the miner
    pub miner_number: u64,
}

impl BlockData {
    pub fn new(nominated_public_key: VerifyingKey, miner_number: u64) -> Self {
        Self {
            nominated_public_key,
            miner_number,
        }
    }
    
    /// Serialize block data to JSON-compatible string for hashing
    pub fn to_hash_string(&self) -> String {
        format!("{}{}", hex::encode(self.nominated_public_key.to_bytes()), self.miner_number)
    }
}

// Serde helper for VerifyingKey
mod public_key_serde {
    use ed25519_dalek::VerifyingKey;
    use serde::{Deserialize, Deserializer, Serializer};
    
    pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(key.to_bytes()))
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let bytes_array: [u8; 32] = bytes.try_into()
            .map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
        VerifyingKey::from_bytes(&bytes_array).map_err(serde::de::Error::custom)
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
        let data = format!("{}{}", self.mining_data(), nonce);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
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
    
    /// Create genesis block (first block in chain)
    pub fn genesis(difficulty: u128, genesis_public_key: VerifyingKey) -> Self {
        let data = BlockData::new(genesis_public_key, 0);
        
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
    use ed25519_dalek::SigningKey;
    
    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[1u8; 32])
    }
    
    #[test]
    fn test_block_data_creation() {
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        let data = BlockData::new(public_key, 12345);
        
        assert_eq!(data.miner_number, 12345);
        assert!(!data.to_hash_string().is_empty());
    }
    
    #[test]
    fn test_genesis_block() {
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        let genesis = Block::genesis(1, public_key);
        
        assert_eq!(genesis.header.index, 0);
        assert_eq!(genesis.header.previous_hash, "0");
        assert_eq!(genesis.data.miner_number, 0);
        assert!(!genesis.header.hash.is_empty());
    }
    
    #[test]
    fn test_data_hash() {
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        let data = BlockData::new(public_key, 42);
        
        let block = Block::new(1, "prev".to_string(), data, 1);
        
        assert!(block.verify_data_hash());
    }
    
    #[test]
    fn test_block_hash_calculation() {
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        let data = BlockData::new(public_key, 100);
        
        let block = Block::new(1, "prev".to_string(), data, 1);
        
        let hash1 = block.header.calculate_hash(0);
        let hash2 = block.header.calculate_hash(0);
        let hash3 = block.header.calculate_hash(1);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}

