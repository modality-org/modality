use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage_path = std::env::args()
        .nth(1)
        .expect("Usage: show_difficulty <storage_path>");
    let datastore = DatastoreManager::open_readonly(std::path::Path::new(&storage_path))?;
    let blocks_per_epoch = datastore.epoch_config().blocks_per_epoch.max(1);

    let mut blocks = MinerBlock::find_all_canonical_multi(&datastore).await?;
    blocks.sort_by_key(|b| b.index);

    if blocks.is_empty() {
        println!("No blocks found.");
        return Ok(());
    }

    println!("Block Index | Epoch |     Target | Change");
    println!("------------|-------|------------|-------");

    let mut last_difficulty = None;
    for block in blocks {
        let difficulty = block.target_difficulty.parse::<u128>().unwrap_or(0);
        let change = if let Some(last) = last_difficulty {
            if difficulty > last {
                format!("+{} (▲)", difficulty - last)
            } else if difficulty < last {
                format!("-{} (▼)", last - difficulty)
            } else {
                "0 (=)".to_string()
            }
        } else {
            "-".to_string()
        };

        println!(
            "{:11} | {:5} | {:10} | {}",
            block.index, block.epoch, difficulty, change
        );
        last_difficulty = Some(difficulty);

        // Print epoch boundary markers
        if block.index > 0 && block.index % blocks_per_epoch == blocks_per_epoch - 1 {
            println!("------------|-------|------------|-------");
        }
    }

    Ok(())
}
