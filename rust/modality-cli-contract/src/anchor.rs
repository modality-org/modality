use anyhow::Result;
use clap::Parser;
#[cfg(feature = "p2p")]
use serde_json::{json, Value};
use std::path::PathBuf;

use modality_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Anchor commit hashes on the network's hash lane, without their bodies")]
pub struct Opts {
    /// Target node multiaddress (defaults to the remote's)
    #[clap(long)]
    remote: Option<String>,

    /// Remote name (default: origin)
    #[clap(long, default_value = "origin")]
    remote_name: String,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Node directory for config (optional, for identity)
    #[clap(long)]
    node_dir: Option<PathBuf>,

    /// Commit to anchor; repeat for several. Defaults to the commits not yet
    /// pushed to the remote.
    #[clap(long = "commit")]
    commits: Vec<String>,

    /// Passfile path or identity name that signs the hash commitments.
    /// Defaults to a one-off key: the signature binds the proof to the
    /// record, not to an author.
    #[clap(long)]
    sign: Option<String>,

    /// Leading zero bits of work to grind for; more work beats other records
    /// when a block is full. Defaults to the network's floor.
    #[clap(long)]
    bits: Option<u32>,

    /// Show what the node indexed for the commits instead of anchoring them
    #[clap(long)]
    status: bool,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let contract_dir = match &opts.dir {
        Some(d) => d.clone(),
        None => std::env::current_dir()?,
    };
    let store = ContractStore::open(&contract_dir)?;
    let config = store.load_config()?;
    let remote_url = match &opts.remote {
        Some(url) => url.clone(),
        None => config
            .get_remote(&opts.remote_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Remote '{}' not found. Use --remote to specify.",
                    opts.remote_name
                )
            })?
            .url
            .clone(),
    };
    let commits = if opts.commits.is_empty() {
        store.get_unpushed_commits(&opts.remote_name)?
    } else {
        opts.commits.clone()
    };
    if commits.is_empty() {
        anyhow::bail!("No commits to anchor: every commit is pushed. Name one with --commit.");
    }

    #[cfg(feature = "p2p")]
    {
        let results = if opts.status {
            status(opts, &remote_url, &config.contract_id, &commits).await?
        } else {
            anchor(opts, &store, &remote_url, &config.contract_id, &commits).await?
        };
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "contract_id": config.contract_id,
                "commits": results,
            }))?);
        } else {
            for r in &results {
                println!(
                    "{}  {}",
                    r["status"].as_str().unwrap_or("?"),
                    r["commit_id"].as_str().unwrap_or("?")
                );
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "p2p"))]
    {
        let _ = (remote_url, commits);
        anyhow::bail!("Anchoring needs the `p2p` feature.")
    }
}

#[cfg(feature = "p2p")]
async fn request(opts: &Opts, remote_url: &str, path: &str, data: &Value) -> Result<Value> {
    let response =
        crate::push::p2p_request(opts.node_dir.as_ref(), remote_url, path, data).await?;
    if !response.ok {
        let why = response
            .errors
            .as_ref()
            .and_then(|e| e.get("error"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{:?}", response.errors));
        anyhow::bail!("{path} refused: {why}");
    }
    Ok(response.data.unwrap_or(Value::Null))
}

#[cfg(feature = "p2p")]
async fn status(
    opts: &Opts,
    remote_url: &str,
    contract_id: &str,
    commits: &[String],
) -> Result<Vec<Value>> {
    let mut results = Vec::new();
    for commit_id in commits {
        results.push(
            request(
                opts,
                remote_url,
                "/hash_commitment/get",
                &json!({"contract_id": contract_id, "commit_id": commit_id}),
            )
            .await?,
        );
    }
    Ok(results)
}

#[cfg(feature = "p2p")]
async fn anchor(
    opts: &Opts,
    store: &ContractStore,
    remote_url: &str,
    contract_id: &str,
    commits: &[String],
) -> Result<Vec<Value>> {
    use modality_common::hash_commitment::{EpochAnchor, HashCommitment, HashLaneParams};
    use modality_common::keypair::Keypair;

    let lane = request(opts, remote_url, "/hash_commitment/params", &json!({})).await?;
    if lane.get("enabled").and_then(Value::as_bool) != Some(true) {
        anyhow::bail!("This network has no hash lane.");
    }
    let params: HashLaneParams = serde_json::from_value(lane["params"].clone())?;
    let current: EpochAnchor = serde_json::from_value(lane["current"].clone())
        .map_err(|_| anyhow::anyhow!("The node has no current epoch anchor yet; retry shortly."))?;
    let bits = opts.bits.unwrap_or(params.floor_bits).max(params.floor_bits);

    let keypair = match &opts.sign {
        Some(reference) => {
            let path = modality_common::passfile::resolve_passfile_path(reference)?;
            let keypair = Keypair::from_json_file(path.to_str().unwrap_or_default())?;
            if !keypair.can_sign() {
                anyhow::bail!("{reference} has no usable private key");
            }
            keypair
        }
        None => Keypair::generate()?,
    };
    // A contract created with --signer posts its set on the genesis record.
    let signer_set = crate::signer_set::SignerSet::load(store)?;
    if let Some(set) = &signer_set {
        if !set.signers.contains(&keypair.public_key_as_base58_identity()) {
            anyhow::bail!(
                "This contract's hash commitments must be signed by one of its signers ({}); pass --sign",
                set.signers.join(", ")
            );
        }
    }

    let mut results = Vec::new();
    for commit_id in commits {
        let parent = store.load_commit(commit_id)?.head.parent;
        let genesis_set = signer_set
            .as_ref()
            .filter(|set| parent.is_none() && &set.genesis_commit_id == commit_id);
        let mut record = if let Some(set) = genesis_set {
            HashCommitment::signed_genesis(
                &keypair,
                contract_id,
                commit_id,
                set.signers.clone(),
                set.contract_signature.clone(),
            )?
        } else {
            HashCommitment::signed(&keypair, contract_id, commit_id, parent.as_deref())?
        };
        record.grind(&current, bits)?;
        let answer = request(
            opts,
            remote_url,
            "/hash_commitment/submit",
            &serde_json::to_value(&record)?,
        )
        .await?;
        results.push(json!({
            "commit_id": commit_id,
            "status": answer.get("status").cloned().unwrap_or(json!("queued")),
            "work_bits": record.work_bits(),
            "anchor_epoch": record.anchor_epoch,
        }));
    }
    Ok(results)
}
