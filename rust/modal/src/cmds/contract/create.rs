use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::keypair::Keypair;
use modal_common::contract_store::{ContractStore, CommitFile};

#[derive(Debug, Parser)]
#[command(about = "Create a new contract in a directory")]
pub struct Opts {
    /// Directory path where the contract will be created (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine the contract directory
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Generate a keypair for the contract
    let keypair = Keypair::generate()?;
    let contract_id = keypair.as_public_address();

    // Initialize the contract store
    let store = ContractStore::init(&dir, contract_id.clone())?;

    // Create model directory with default model
    let model_dir = dir.join("model");
    std::fs::create_dir_all(&model_dir)?;
    
    let default_model = r#"export default model {
  init --> init
}
"#;
    std::fs::write(model_dir.join("default.modality"), default_model)?;

    // Create genesis commit
    let genesis = serde_json::json!({
        "genesis": {
            "contract_id": contract_id.clone(),
            "created_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            "public_key": keypair.public_key_as_base58_identity()
        }
    });

    // Save genesis
    store.save_genesis(&genesis)?;

    // Create initial genesis commit as HEAD
    let mut genesis_commit = CommitFile::new();
    genesis_commit.add_action(
        "genesis".to_string(),
        None,
        genesis.clone()
    );
    
    let genesis_commit_id = genesis_commit.compute_id()?;
    store.save_commit(&genesis_commit_id, &genesis_commit)?;
    store.set_head(&genesis_commit_id)?;

    // Output
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "contract_id": contract_id,
            "directory": dir.display().to_string(),
            "genesis_commit_id": genesis_commit_id,
        }))?);
    } else {
        println!("✅ Contract created successfully!");
        println!("   Contract ID: {}", contract_id);
        println!("   Directory: {}", dir.display());
        println!("   Genesis commit: {}", genesis_commit_id);
        println!();
        println!("Next steps:");
        println!("  1. cd {}", dir.display());
        println!("  2. Edit model/default.modality to define your state machine");
        println!("  3. Add rules in rules/*.modality");
        println!("  4. modal c commit --all --sign your.passfile");
    }

    Ok(())
}
