use anyhow::Result;
use clap::Parser;
use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;
use modal_observer::{ChainObserver, ForkConfig};
use modal_miner::block::{Block, BlockData};
use modal_miner::miner::Miner;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
pub struct Opts {
    /// Specific test(s) to run (fork, gap, missing-parent, integrity, promotion, duplicate-canonical)
    /// Can be specified multiple times. If not specified, runs all tests.
    #[arg(short, long = "test", value_name = "TEST")]
    tests: Vec<String>,
    
    /// Path to existing datastore directory (if not specified, uses in-memory)
    #[arg(short, long, value_name = "PATH")]
    datastore: Option<PathBuf>,
    
    /// Output results in JSON format
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    test: String,
    status: String,
    message: String,
    orphan_reason: Option<String>,
    details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestSummary {
    results: Vec<TestResult>,
    summary: Summary,
}

#[derive(Debug, Serialize, Deserialize)]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
}

/// Helper to create and mine a block with specific properties
/// Uses difficulty=1 for fast testing
fn create_and_mine_block(
    index: u64,
    previous_hash: String,
    nominated_peer_id: String,
    miner_number: u64,
) -> Block {
    let block_data = BlockData::new(nominated_peer_id, miner_number);
    let block = Block::new(index, previous_hash, block_data, 1);
    let miner = Miner::new_default();
    miner.mine_block(block).expect("Mining should succeed")
}

/// Convert a Block to MinerBlock for use with ChainObserver
fn block_to_miner_block(block: &Block) -> MinerBlock {
    let epoch = block.header.index / 40;
    MinerBlock::new_canonical(
        block.header.hash.clone(),
        block.header.index,
        epoch,
        block.header.timestamp.timestamp(),
        block.header.previous_hash.clone(),
        block.header.data_hash.clone(),
        block.header.nonce,
        block.header.difficulty,
        block.data.nominated_peer_id.clone(),
        block.data.miner_number,
    )
}

/// Test: Fork Detection - Two blocks at the same index
async fn test_fork_detection(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    let block_1a = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
    let block_1a_mb = block_to_miner_block(&block_1a);
    observer.process_gossiped_block(block_1a_mb).await?;
    
    let block_1b = create_and_mine_block(1, genesis.header.hash.clone(), "peer_b".to_string(), 2);
    let block_1b_mb = block_to_miner_block(&block_1b);
    let accepted = observer.process_gossiped_block(block_1b_mb).await?;
    
    if accepted {
        return Ok(TestResult {
            test: "fork".to_string(),
            status: "failed".to_string(),
            message: "Second block at same index was incorrectly accepted".to_string(),
            orphan_reason: None,
            details: None,
        });
    }
    
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_1b.header.hash).await?;
    drop(ds);
    
    match orphaned {
        Some(block) if block.is_orphaned => {
            let reason = block.orphan_reason.clone().unwrap_or_default();
            Ok(TestResult {
                test: "fork".to_string(),
                status: "passed".to_string(),
                message: "Correctly identified competing block at same index".to_string(),
                orphan_reason: Some(reason),
                details: None,
            })
        }
        _ => Ok(TestResult {
            test: "fork".to_string(),
            status: "failed".to_string(),
            message: "Orphaned block not stored correctly".to_string(),
            orphan_reason: None,
            details: None,
        }),
    }
}

/// Test: Gap Detection - Block arrives with missing parent index
async fn test_gap_detection(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    observer.process_gossiped_block(block_1_mb).await?;
    
    let block_3 = create_and_mine_block(3, block_1.header.hash.clone(), "peer_a".to_string(), 3);
    let block_3_mb = block_to_miner_block(&block_3);
    let accepted = observer.process_gossiped_block(block_3_mb).await?;
    
    if accepted {
        return Ok(TestResult {
            test: "gap".to_string(),
            status: "failed".to_string(),
            message: "Block with gap was incorrectly accepted".to_string(),
            orphan_reason: None,
            details: None,
        });
    }
    
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_3.header.hash).await?;
    drop(ds);
    
    match orphaned {
        Some(block) if block.is_orphaned => {
            let reason = block.orphan_reason.clone().unwrap_or_default();
            Ok(TestResult {
                test: "gap".to_string(),
                status: "passed".to_string(),
                message: "Correctly identified missing block in chain".to_string(),
                orphan_reason: Some(reason),
                details: Some("Gap between index 1 and 3".to_string()),
            })
        }
        _ => Ok(TestResult {
            test: "gap".to_string(),
            status: "failed".to_string(),
            message: "Orphaned block not stored correctly".to_string(),
            orphan_reason: None,
            details: None,
        }),
    }
}

/// Test: Missing Parent - Block references unknown parent hash
async fn test_missing_parent(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    let fake_parent_hash = "deadbeef".repeat(8);
    let block_1 = create_and_mine_block(1, fake_parent_hash, "peer_a".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    let accepted = observer.process_gossiped_block(block_1_mb).await?;
    
    if accepted {
        return Ok(TestResult {
            test: "missing-parent".to_string(),
            status: "failed".to_string(),
            message: "Block with unknown parent was incorrectly accepted".to_string(),
            orphan_reason: None,
            details: None,
        });
    }
    
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_1.header.hash).await?;
    drop(ds);
    
    match orphaned {
        Some(block) if block.is_orphaned => {
            let reason = block.orphan_reason.clone().unwrap_or_default();
            Ok(TestResult {
                test: "missing-parent".to_string(),
                status: "passed".to_string(),
                message: "Correctly identified unknown parent hash".to_string(),
                orphan_reason: Some(reason),
                details: None,
            })
        }
        _ => Ok(TestResult {
            test: "missing-parent".to_string(),
            status: "failed".to_string(),
            message: "Orphaned block not stored correctly".to_string(),
            orphan_reason: None,
            details: None,
        }),
    }
}

/// Test: Chain Integrity - Verify canonical chain remains consistent
async fn test_chain_integrity(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    let genesis = Block::genesis(1, "genesis".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
    observer.process_gossiped_block(block_to_miner_block(&block_1)).await?;
    
    let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
    observer.process_gossiped_block(block_to_miner_block(&block_2)).await?;
    
    let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
    observer.process_gossiped_block(block_to_miner_block(&block_3)).await?;
    
    // Add forks
    let fork_1 = create_and_mine_block(1, genesis.header.hash.clone(), "fork1".to_string(), 10);
    observer.process_gossiped_block(block_to_miner_block(&fork_1)).await?;
    
    let fork_2 = create_and_mine_block(2, block_1.header.hash.clone(), "fork2".to_string(), 20);
    observer.process_gossiped_block(block_to_miner_block(&fork_2)).await?;
    
    let ds = datastore.lock().await;
    let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
    let all_blocks = MinerBlock::find_all_blocks_multi(&ds).await?;
    drop(ds);
    
    let orphaned_count = all_blocks.iter().filter(|b| b.is_orphaned).count();
    
    if canonical_blocks.len() == 4 && orphaned_count == 2 {
        Ok(TestResult {
            test: "integrity".to_string(),
            status: "passed".to_string(),
            message: "Canonical chain remains consistent after orphaning".to_string(),
            orphan_reason: None,
            details: Some(format!("Canonical: {}, Orphaned: {}", canonical_blocks.len(), orphaned_count)),
        })
    } else {
        Ok(TestResult {
            test: "integrity".to_string(),
            status: "failed".to_string(),
            message: format!("Unexpected chain state: {} canonical, {} orphaned", 
                canonical_blocks.len(), orphaned_count),
            orphan_reason: None,
            details: None,
        })
    }
}

/// Test: Orphan Promotion - When missing parent arrives, orphan can be promoted
async fn test_orphan_promotion(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    let genesis = Block::genesis(1, "genesis".to_string());
    observer.process_gossiped_block(block_to_miner_block(&genesis)).await?;
    
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
    observer.process_gossiped_block(block_to_miner_block(&block_1)).await?;
    
    let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
    let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
    let block_3_mb = block_to_miner_block(&block_3);
    
    // Block 3 should be orphaned (missing block 2)
    observer.process_gossiped_block(block_3_mb.clone()).await?;
    
    // Add block 2
    observer.process_gossiped_block(block_to_miner_block(&block_2)).await?;
    
    // Re-submit block 3 (should be promoted)
    let accepted = observer.process_gossiped_block(block_3_mb).await?;
    
    if !accepted {
        return Ok(TestResult {
            test: "promotion".to_string(),
            status: "failed".to_string(),
            message: "Orphan was not promoted when parent arrived".to_string(),
            orphan_reason: None,
            details: None,
        });
    }
    
    let ds = datastore.lock().await;
    let block_3_final = MinerBlock::find_by_hash_multi(&ds, &block_3.header.hash).await?;
    drop(ds);
    
    match block_3_final {
        Some(block) if block.is_canonical && !block.is_orphaned => {
            Ok(TestResult {
                test: "promotion".to_string(),
                status: "passed".to_string(),
                message: "Successfully promoted orphan when parent arrived".to_string(),
                orphan_reason: None,
                details: None,
            })
        }
        _ => Ok(TestResult {
            test: "promotion".to_string(),
            status: "failed".to_string(),
            message: "Block not properly promoted to canonical".to_string(),
            orphan_reason: None,
            details: None,
        }),
    }
}

/// Test: Duplicate Canonical Detection - Check for duplicate canonical blocks
async fn test_duplicate_canonical(datastore: Arc<Mutex<DatastoreManager>>) -> Result<TestResult> {
    let ds = datastore.lock().await;
    let duplicates = modal_datastore::models::miner::integrity::detect_duplicate_canonical_blocks_multi(&ds).await?;
    drop(ds);
    
    if duplicates.is_empty() {
        Ok(TestResult {
            test: "duplicate-canonical".to_string(),
            status: "passed".to_string(),
            message: "No duplicate canonical blocks found".to_string(),
            orphan_reason: None,
            details: None,
        })
    } else {
        let indices: Vec<String> = duplicates.iter()
            .map(|d| format!("{} ({} blocks)", d.index, d.blocks.len()))
            .collect();
        Ok(TestResult {
            test: "duplicate-canonical".to_string(),
            status: "failed".to_string(),
            message: format!("Found {} indices with duplicate canonical blocks", duplicates.len()),
            orphan_reason: None,
            details: Some(format!("Affected indices: {}", indices.join(", "))),
        })
    }
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine which tests to run
    let all_tests = vec!["fork", "gap", "missing-parent", "integrity", "promotion", "duplicate-canonical"];
    let tests_to_run: Vec<&str> = if opts.tests.is_empty() {
        all_tests.clone()
    } else {
        opts.tests.iter().map(|s| s.as_str()).collect()
    };
    
    // Run tests
    let mut results = Vec::new();
    
    for test_name in &tests_to_run {
        // Create a fresh datastore for each test (or use the provided one)
        let datastore = if let Some(path) = &opts.datastore {
            // Use the provided datastore for all tests
            Arc::new(Mutex::new(DatastoreManager::open(path)?))
        } else {
            // Create a fresh in-memory datastore for each test
            Arc::new(Mutex::new(DatastoreManager::create_in_memory()?))
        };
        
        let result = match *test_name {
            "fork" => test_fork_detection(datastore).await?,
            "gap" => test_gap_detection(datastore).await?,
            "missing-parent" => test_missing_parent(datastore).await?,
            "integrity" => test_chain_integrity(datastore).await?,
            "promotion" => test_orphan_promotion(datastore).await?,
            "duplicate-canonical" => test_duplicate_canonical(datastore).await?,
            _ => {
                eprintln!("Unknown test: {}", test_name);
                continue;
            }
        };
        results.push(result);
    }
    
    // Calculate summary
    let passed = results.iter().filter(|r| r.status == "passed").count();
    let failed = results.len() - passed;
    
    // Output results
    if opts.json {
        let summary = TestSummary {
            results,
            summary: Summary {
                total: passed + failed,
                passed,
                failed,
            },
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("===========================================");
        println!("  Chain Validation Results");
        println!("===========================================");
        println!();
        
        for result in &results {
            let icon = if result.status == "passed" { "✅" } else { "❌" };
            let test_name = match result.test.as_str() {
                "fork" => "Fork Detection",
                "gap" => "Gap Detection",
                "missing-parent" => "Missing Parent Detection",
                "integrity" => "Chain Integrity",
                "promotion" => "Orphan Promotion",
                "duplicate-canonical" => "Duplicate Canonical Detection",
                _ => &result.test,
            };
            
            println!("{} {}", icon, test_name);
            println!("   {}", result.message);
            if let Some(reason) = &result.orphan_reason {
                println!("   Orphan reason: {}", reason);
            }
            if let Some(details) = &result.details {
                println!("   Details: {}", details);
            }
            println!();
        }
        
        println!("===========================================");
        println!("Passed: {}/{}", passed, passed + failed);
        println!("===========================================");
    }
    
    if failed > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

