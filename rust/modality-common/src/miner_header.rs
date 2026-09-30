//! How a miner block header is hashed: the text its proof of work covers,
//! and the hash of the data it commits to. `modality-miner` builds blocks
//! with these, and the `mined_headers` predicate checks headers with them.

use sha2::{Digest, Sha256};

/// The text a block's nonce is ground against.
pub fn mining_data(
    index: u64,
    timestamp: i64,
    previous_hash: &str,
    data_hash: &str,
    difficulty: u128,
) -> String {
    format!("{index}{timestamp}{previous_hash}{data_hash}{difficulty}")
}

/// The hash of a block's data: its nominee and the miner's number.
pub fn data_hash(nominated_peer_id: &str, miner_number: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{nominated_peer_id}{miner_number}").as_bytes());
    format!("{:x}", hasher.finalize())
}
