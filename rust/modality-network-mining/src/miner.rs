use crate::block::Block;
use crate::error::MiningError;
use modality_utils::hash_tax;

/// Configuration for the miner
#[derive(Debug, Clone)]
pub struct MinerConfig {
    pub max_tries: Option<u128>,
    pub hash_func_name: Option<&'static str>,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            max_tries: None,
            hash_func_name: Some("sha256"),
        }
    }
}

/// Miner for proof-of-work blockchain
#[derive(Debug, Clone)]
pub struct Miner {
    config: MinerConfig,
}

impl Miner {
    pub fn new(config: MinerConfig) -> Self {
        Self { config }
    }
    
    pub fn new_default() -> Self {
        Self::new(MinerConfig::default())
    }
    
    /// Mine a block by finding a valid nonce
    pub fn mine_block(&self, mut block: Block) -> Result<Block, MiningError> {
        let mining_data = block.mining_data();
        let difficulty = block.header.difficulty;
        
        // Use hash_tax to find a valid nonce
        let nonce = hash_tax::mine(
            &mining_data,
            difficulty,
            self.config.max_tries,
            self.config.hash_func_name,
        )
        .map_err(|e| MiningError::MiningFailed(e.to_string()))?;
        
        // Update block with found nonce and hash
        block.header.nonce = nonce;
        block.header.hash = block.header.calculate_hash(nonce);
        
        Ok(block)
    }
    
    /// Verify a mined block's nonce is valid
    pub fn verify_block(&self, block: &Block) -> Result<bool, MiningError> {
        // Verify hash is correct
        if !block.verify_hash() {
            return Ok(false);
        }
        
        // Genesis block (index 0) is always valid if hash is correct
        if block.header.index == 0 {
            return Ok(true);
        }
        
        let mining_data = block.mining_data();
        let nonce = block.header.nonce;
        let difficulty = block.header.difficulty;
        
        // Verify nonce meets difficulty using hash_tax
        hash_tax::validate_nonce(
            &mining_data,
            nonce,
            difficulty,
            self.config.hash_func_name.unwrap_or("sha256"),
        )
        .map_err(|e| MiningError::HashError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockData;
    use ed25519_dalek::SigningKey;
    
    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[1u8; 32])
    }
    
    #[test]
    fn test_mine_block() {
        let miner = Miner::new_default();
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        
        let data = BlockData::new(public_key, 12345);
        let block = Block::new(1, "prev_hash".to_string(), data, 100);
        
        let mined_block = miner.mine_block(block).unwrap();
        
        assert!(miner.verify_block(&mined_block).unwrap());
        assert!(!mined_block.header.hash.is_empty());
        assert!(mined_block.header.nonce > 0);
        assert_eq!(mined_block.data.miner_number, 12345);
    }
    
    #[test]
    fn test_verify_invalid_block() {
        let miner = Miner::new_default();
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        
        let data = BlockData::new(public_key, 100);
        let mut block = Block::new(1, "prev_hash".to_string(), data, 100);
        block.header.nonce = 12345;
        block.header.hash = "invalid_hash".to_string();
        
        assert!(!miner.verify_block(&block).unwrap());
    }
    
    #[test]
    fn test_genesis_block_verification() {
        let miner = Miner::new_default();
        let signing_key = test_signing_key();
        let public_key = signing_key.verifying_key();
        let genesis = Block::genesis(1, public_key);
        
        // Genesis block has difficulty 1 and nonce 0, should be valid
        assert!(miner.verify_block(&genesis).unwrap());
    }
}

