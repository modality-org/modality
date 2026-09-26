//! Score a copied node store the way fork choice does, and say why the
//! stored spine wins or loses against the canonical chain.
//!
//! Usage: `cargo run -p modality-node --example score_store -- <data_dir>`

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use modality_node::chain::fork_choice::{
    check_expected_target_indexed, index_by_hash, score_canonical_chain, RetargetParams,
    TargetCheck,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: score_store <data_dir>"))?;
    let mgr = DatastoreManager::open(std::path::Path::new(&dir))?;
    let params = RetargetParams {
        blocks_per_epoch: mgr.epoch_config().blocks_per_epoch,
        target_block_time_secs: mgr.network_u64("target_block_time_secs").unwrap_or(60),
        initial_difficulty: mgr
            .network_u64("initial_difficulty")
            .map(|value| value as u128),
    };
    println!("params: {params:?}");
    let t0 = std::time::Instant::now();
    let all = MinerBlock::find_all_blocks_multi(&mgr).await?;
    println!("load {} blocks: {:?}", all.len(), t0.elapsed());
    let t1 = std::time::Instant::now();
    let live: Vec<MinerBlock> = all.into_iter().filter(|b| !b.is_orphaned).collect();
    let mut eligible = Vec::new();
    let by_hash = index_by_hash(&live);
    for block in &live {
        match check_expected_target_indexed(block, &by_hash, params) {
            TargetCheck::Reject => println!(
                "REJECT {} {} target {}",
                block.index,
                &block.hash[..12],
                block.target_difficulty
            ),
            TargetCheck::Unresolved => {
                println!(
                    "unresolved {} {} target {}",
                    block.index,
                    &block.hash[..12],
                    block.target_difficulty
                );
                eligible.push(block.clone());
            }
            TargetCheck::Matches => eligible.push(block.clone()),
        }
    }
    println!("target checks over {} live: {:?}", live.len(), t1.elapsed());
    let t2 = std::time::Instant::now();
    let winner = MinerBlock::verified_spine(&eligible);
    println!("verified_spine: {:?}", t2.elapsed());
    let canonical: Vec<MinerBlock> = live.iter().filter(|b| b.is_canonical).cloned().collect();
    let ws = score_canonical_chain(&winner);
    let ls = score_canonical_chain(&canonical);
    println!(
        "winner: len {} tip {} {} work {:?}",
        winner.len(),
        ws.tip,
        &ws.tip_hash[..12.min(ws.tip_hash.len())],
        ws.work
    );
    println!(
        "local canonical: len {} tip {} {} work {:?}",
        canonical.len(),
        ls.tip,
        &ls.tip_hash[..12.min(ls.tip_hash.len())],
        ls.work
    );
    Ok(())
}
