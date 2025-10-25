use modal_datastore::{NetworkDatastore, Model};
use modal_datastore::models::MinerBlock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage_path = std::env::args().nth(1).expect("Usage: show_difficulty <storage_path>");
    let datastore = NetworkDatastore::open(&storage_path)?;
    
    let blocks = MinerBlock::find_all_canonical(&datastore).await?;
    
    if blocks.is_empty() {
        println!("No blocks found.");
        return Ok(());
    }
    
    println!("Block Index | Epoch | Difficulty | Change");
    println!("------------|-------|------------|-------");
    
    let mut last_difficulty = None;
    for block in blocks {
        let difficulty = block.difficulty.parse::<u128>().unwrap_or(0);
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
        
        println!("{:11} | {:5} | {:10} | {}", block.index, block.epoch, difficulty, change);
        last_difficulty = Some(difficulty);
        
        // Print epoch boundary markers
        if block.index > 0 && block.index % 40 == 39 {
            println!("------------|-------|------------|-------");
        }
    }
    
    Ok(())
}
