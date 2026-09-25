//! Dump every stored miner block as one JSON line.
//!
//! Usage: `cargo run -p modality-datastore --example dump_miner_blocks -- <data_dir>`

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: dump_miner_blocks <data_dir>"))?;
    let mgr = DatastoreManager::open(std::path::Path::new(&dir))?;
    let blocks = MinerBlock::find_all_blocks_multi(&mgr).await?;
    for block in blocks {
        println!(
            "{}",
            serde_json::json!({
                "index": block.index,
                "hash": block.hash,
                "previous_hash": block.previous_hash,
                "epoch": block.epoch,
                "target": block.target_difficulty,
                "actual": block.actualized_difficulty,
                "canonical": block.is_canonical,
                "orphaned": block.is_orphaned,
                "reason": block.orphan_reason,
                "timestamp": block.timestamp,
                "nominee": block.nominated_peer_id,
            })
        );
    }
    Ok(())
}
