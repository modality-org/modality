//! Time the fork-choice and startup reads a node makes, on a copied store.
//!
//! Usage: `cargo run -p modality-node --example replay_store -- <data_dir>`
//!
//! Opens the store, then times the miner cache load, a tree sync, a select
//! pass, an adoption pass, the startup tip read, and one more select pass
//! (which should be cheap). Nothing leaves the machine.

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let dir = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: replay_store <data_dir>"))?;
    let mut mgr = DatastoreManager::open(std::path::Path::new(&dir))?;
    mgr.set_blocks_per_epoch(40);

    let t = Instant::now();
    let all = MinerBlock::find_all_blocks_multi(&mgr).await?;
    let live = all.iter().filter(|b| !b.is_orphaned).count();
    let canonical = all.iter().filter(|b| b.is_canonical && !b.is_orphaned).count();
    println!(
        "cache load: {} blocks ({live} live, {canonical} canonical) in {:?}",
        all.len(),
        t.elapsed()
    );

    let params = modality_node::chain::tree_choice::retarget_params(&mgr);
    let t = Instant::now();
    let (best, cbest, leaves) = modality_node::chain::tree_choice::with_synced_tree(&mgr, params, |tree| {
        (
            tree.best().map(|(b, w)| (b.index, b.hash[..12].to_string(), w)),
            tree.canonical_best().map(|(b, w)| (b.index, b.hash[..12].to_string(), w)),
            tree.linked_leaves().len(),
        )
    })?;
    println!("tree sync: {:?}\n  winner {best:?}\n  canonical {cbest:?}\n  linked leaves {leaves}", t.elapsed());

    let t = Instant::now();
    let switched = modality_node::chain::reorg::select_best_stored_chain(&mgr).await?;
    println!("select: {switched} in {:?}", t.elapsed());
    let t = Instant::now();
    let adopted = modality_node::chain::reorg::adopt_connected_extensions(&mgr).await?;
    println!("adopt: {adopted} in {:?}", t.elapsed());

    let t = Instant::now();
    let tip = MinerBlock::verified_spine(&MinerBlock::find_all_canonical_multi(&mgr).await?)
        .last()
        .map(|b| b.index);
    println!("startup tip read: {tip:?} in {:?}", t.elapsed());

    let t = Instant::now();
    let switched = modality_node::chain::reorg::select_best_stored_chain(&mgr).await?;
    println!("second select: {switched} in {:?}", t.elapsed());

    // What parent-hash sync computes over every stored block.
    let t = Instant::now();
    let all = MinerBlock::find_all_blocks_multi(&mgr).await?;
    let missing = modality_node::chain::reorg::missing_parent_hashes(&all);
    println!("missing_parent_hashes: {} gaps in {:?}", missing.len(), t.elapsed());

    // The promotion task's first pass, as it runs at startup.
    let epoch = MinerBlock::find_all_canonical_multi(&mgr)
        .await?
        .into_iter()
        .max_by_key(|b| b.index)
        .map(|b| b.epoch)
        .unwrap_or(0);
    let t = Instant::now();
    let promoted = MinerBlock::run_promotion(&mgr, epoch).await?;
    println!("promotion: {promoted:?} in {:?}", t.elapsed());
    let t = Instant::now();
    let purged = MinerBlock::run_purge(&mgr, epoch).await?;
    println!("purge: {purged} in {:?}", t.elapsed());
    let t = Instant::now();
    let pruned =
        modality_node::actions::observer::chain_maintenance::prune_under_sequenced_prefix(&mgr)
            .await?;
    println!("prune: {pruned} in {:?}", t.elapsed());
    Ok(())
}
