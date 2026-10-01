use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modality_common::contract_store::{CommitFile, ContractStore};
use modality_common::hash_commitment::{creation_rule, sign_signer_set, signer_path};
use modality_common::keypair::Keypair;

use crate::signer_set::SignerSet;

#[derive(Debug, Parser)]
#[command(about = "Create a new contract in a directory")]
pub struct Opts {
    /// Directory path where the contract will be created (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// A key allowed to extend this contract (Modality ID, passfile path, or
    /// identity name); repeat for several. The genesis commit posts the keys
    /// at /signers/<n>.id with a rule that every later commit is signed by
    /// one of them, and the contract's key signs the set for the hash lane.
    /// Fixed at creation.
    #[clap(long = "signer")]
    signers: Vec<String>,

    /// The contract's key, from a passfile (path or identity name), instead
    /// of a new one: the contract's id is that key's id. A miner makes the
    /// contract its blocks nominate this way, to take their MOD. One
    /// contract per key.
    #[clap(long)]
    key: Option<String>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

/// A contract made by [`make`].
pub struct Created {
    pub contract_id: String,
    pub dir: std::path::PathBuf,
    pub genesis_commit_id: String,
    pub signers: Vec<String>,
}

/// Create the contract `opts` describes, without printing.
pub async fn make(opts: &Opts) -> Result<Created> {
    // Determine the contract directory
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    let signers = opts
        .signers
        .iter()
        .map(|reference| crate::signer_set::signer_id(reference))
        .collect::<Result<Vec<_>>>()?;
    for (i, id) in signers.iter().enumerate() {
        if signers[..i].contains(id) {
            anyhow::bail!("--signer {id} is given twice");
        }
    }

    let keypair = match &opts.key {
        Some(reference) => {
            let path = modality_common::passfile::resolve_passfile_path(reference)?;
            Keypair::from_json_file(path.to_str().unwrap_or_default())
                .map_err(|e| anyhow::anyhow!("--key {reference}: {e}"))?
        }
        None => Keypair::generate()?,
    };
    let contract_id = keypair.as_public_address();

    // Initialize the contract store
    let store = ContractStore::init(&dir, contract_id.clone())?;

    // Create model directory with default model
    let model_dir = dir.join("model");
    std::fs::create_dir_all(&model_dir)?;

    let paths: Vec<String> = (1..=signers.len()).map(signer_path).collect();
    let default_model = if signers.is_empty() {
        "export default model {\n  init --> init\n}\n".to_string()
    } else {
        // Genesis takes the first step; every step after it is signed.
        let edges: String = paths
            .iter()
            .map(|p| format!("  signed --> signed: +signed_by({p})\n"))
            .collect();
        format!("export default model {{\n  init --> signed\n{edges}}}\n")
    };
    std::fs::write(model_dir.join("default.modality"), &default_model)?;

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
    genesis_commit.add_action("genesis".to_string(), None, genesis.clone());
    if !signers.is_empty() {
        // The creation rule, with the keys it names and a model that meets it.
        for (path, id) in paths.iter().zip(&signers) {
            genesis_commit.add_action("post".to_string(), Some(path.clone()), id.clone().into());
        }
        genesis_commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            default_model.clone().into(),
        );
        genesis_commit.add_action(
            "rule".to_string(),
            Some("/rules/signers.modality".to_string()),
            crate::add_rule::format_rule(&creation_rule(&paths), "$PARENT").into(),
        );
    }

    let genesis_commit_id = genesis_commit.compute_id()?;
    store.save_commit(&genesis_commit_id, &genesis_commit)?;
    store.set_head(&genesis_commit_id)?;
    if !signers.is_empty() {
        store.checkout_state()?;
        // The only time the contract's key is held: sign the set for the hash lane.
        SignerSet {
            genesis_commit_id: genesis_commit_id.clone(),
            contract_signature: sign_signer_set(&keypair, &genesis_commit_id, &signers)?,
            signers: signers.clone(),
        }
        .save(&store)?;
    }

    Ok(Created {
        contract_id,
        dir,
        genesis_commit_id,
        signers,
    })
}

pub async fn run(opts: &Opts) -> Result<()> {
    let Created {
        contract_id,
        dir,
        genesis_commit_id,
        signers,
    } = make(opts).await?;
    let paths: Vec<String> = (1..=signers.len()).map(signer_path).collect();

    // Output
    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contract_id": contract_id,
                "directory": dir.display().to_string(),
                "genesis_commit_id": genesis_commit_id,
                "signers": signers,
            }))?
        );
    } else {
        println!("✅ Contract created successfully!");
        println!("   Contract ID: {}", contract_id);
        println!("   Directory: {}", dir.display());
        println!("   Genesis commit: {}", genesis_commit_id);
        for (path, id) in paths.iter().zip(&signers) {
            println!("   Signer {}: {}", path, id);
        }
        println!();
        println!("Next steps:");
        println!("  1. cd {}", dir.display());
        println!("  2. Edit model/default.modality to define your state machine");
        println!("  3. Add rules in rules/*.modality");
        println!("  4. modal commit --all --sign your.modal_passfile");
    }

    Ok(())
}
