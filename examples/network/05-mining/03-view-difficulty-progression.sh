#!/usr/bin/env bash
cd $(dirname -- "$0")
set -e

echo "📈 Difficulty Progression"
echo "========================"
echo ""

if [ ! -d "./tmp/storage/miner" ]; then
    echo "❌ No miner storage found. Run 01-mine-blocks.sh first."
    exit 1
fi

# Build the modality CLI if needed
if [ ! -f "../../../rust/target/debug/modality" ]; then
    echo "Building modality CLI..."
    cd ../../../rust
    cargo build --package modality
    cd - > /dev/null
fi

# Get all blocks and show difficulty changes
echo "Getting blocks from datastore..."
echo ""

# Use a temporary Rust program to read blocks directly
cat > /tmp/show_difficulty.rs << 'EOF'
use modality_network_datastore::{NetworkDatastore, Model};
use modality_network_datastore::models::MinerBlock;

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
EOF

# Compile and run the temp program
echo "Compiling difficulty viewer..."
cd ../../../rust
cargo build --example show_difficulty 2>/dev/null || {
    # If example doesn't exist, create it
    mkdir -p modality-network-datastore/examples
    cp /tmp/show_difficulty.rs modality-network-datastore/examples/
    cargo build --package modality-network-datastore --example show_difficulty
}

./target/debug/examples/show_difficulty "$(pwd)/../examples/network/05-mining/tmp/storage/miner"

