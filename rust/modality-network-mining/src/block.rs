use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Represents a transaction in a block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: DateTime<Utc>,
    pub data: Option<String>,
}

impl Transaction {
    pub fn new(from: String, to: String, amount: u64, data: Option<String>) -> Self {
        Self {
            from,
            to,
            amount,
            timestamp: Utc::now(),
            data,
        }
    }
    
    /// Serialize transaction to JSON string for hashing
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub index: u64,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: String,
    pub merkle_root: String,
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
            self.merkle_root,
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
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block (before mining)
    pub fn new(
        index: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        difficulty: u128,
    ) -> Self {
        let merkle_root = Self::calculate_merkle_root(&transactions);
        
        let header = BlockHeader {
            index,
            timestamp: Utc::now(),
            previous_hash,
            merkle_root,
            nonce: 0,
            difficulty,
            hash: String::new(),
        };
        
        Self {
            header,
            transactions,
        }
    }
    
    /// Create genesis block (first block in chain)
    pub fn genesis(difficulty: u128) -> Self {
        let mut block = Self::new(
            0,
            "0".to_string(),
            vec![],
            difficulty,
        );
        
        // Genesis block doesn't need mining, just set hash
        block.header.hash = block.header.calculate_hash(0);
        block
    }
    
    /// Calculate Merkle root of transactions
    fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return "0".to_string();
        }
        
        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let mut hasher = Sha256::new();
                hasher.update(tx.to_json_string().as_bytes());
                format!("{:x}", hasher.finalize())
            })
            .collect();
        
        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            
            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left
                };
                
                let combined = format!("{}{}", left, right);
                let mut hasher = Sha256::new();
                hasher.update(combined.as_bytes());
                new_hashes.push(format!("{:x}", hasher.finalize()));
            }
            
            hashes = new_hashes;
        }
        
        hashes[0].clone()
    }
    
    /// Verify the merkle root is correct
    pub fn verify_merkle_root(&self) -> bool {
        let calculated = Self::calculate_merkle_root(&self.transactions);
        calculated == self.header.merkle_root
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
    fn test_transaction_creation() {
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            None,
        );
        
        assert_eq!(tx.from, "alice");
        assert_eq!(tx.to, "bob");
        assert_eq!(tx.amount, 100);
    }
    
    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis(1);
        
        assert_eq!(genesis.header.index, 0);
        assert_eq!(genesis.header.previous_hash, "0");
        assert_eq!(genesis.transactions.len(), 0);
        assert!(!genesis.header.hash.is_empty());
    }
    
    #[test]
    fn test_merkle_root() {
        let tx1 = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            None,
        );
        let tx2 = Transaction::new(
            "bob".to_string(),
            "charlie".to_string(),
            50,
            None,
        );
        
        let block = Block::new(1, "prev".to_string(), vec![tx1, tx2], 1);
        
        assert!(block.verify_merkle_root());
    }
    
    #[test]
    fn test_empty_transactions_merkle_root() {
        let block = Block::new(1, "prev".to_string(), vec![], 1);
        
        assert_eq!(block.header.merkle_root, "0");
        assert!(block.verify_merkle_root());
    }
}

