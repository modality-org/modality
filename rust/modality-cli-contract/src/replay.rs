use anyhow::Result;
use clap::Parser;
use modality_common::contract_store::{CommitFile, ContractStore};
use modality_common::independent_replay::{
    artifact_from_prefix, load_artifact_json, verify_replay_artifact, ReplayArtifact, ReplayReport,
};
use serde_json::json;
use std::path::PathBuf;

#[cfg(feature = "wasm")]
use modality_common::contract_store::CommitAction;
#[cfg(feature = "wasm")]
use modality_common::independent_replay::{
    FrozenInvokeContext, InvokeEngine, ReplayWasm, FROZEN_INVOKE_TIMESTAMP,
};
#[cfg(feature = "wasm")]
use modality_wasm_runtime::WasmExecutor;
#[cfg(feature = "wasm")]
use modality_wasm_validation::{
    decode_program_result, encode_program_input, validate_program_result, ProgramContext,
};
#[cfg(feature = "wasm")]
use serde_json::Value;
#[cfg(feature = "wasm")]
use sha2::{Digest, Sha256};

#[cfg(feature = "p2p")]
use modality_node::actions::request;
#[cfg(feature = "p2p")]
use modality_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Fetch a sequenced prefix and re-check it without the original node")]
pub struct Opts {
    /// Saved replay artifact (offline verify)
    #[clap(long)]
    artifact: Option<PathBuf>,

    /// Write the fetched or local artifact to this path
    #[clap(long)]
    save: Option<PathBuf>,

    /// Sequencer multiaddress to fetch from
    #[clap(long)]
    remote: Option<String>,

    /// Contract id (required with --remote unless --dir has a contract)
    #[clap(long)]
    contract_id: Option<String>,

    /// Sequenced tip to replay through (default: unique sequenced tip)
    #[clap(long)]
    through: Option<String>,

    /// Local contract directory (defaults to current directory when not fetching)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Node directory for identity/config when using P2P remotes
    #[clap(long)]
    node_dir: Option<PathBuf>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let artifact = if let Some(path) = &opts.artifact {
        load_artifact_json(&std::fs::read_to_string(path)?)?
    } else if let Some(remote) = &opts.remote {
        fetch_remote_artifact(opts, remote).await?
    } else {
        artifact_from_local_dir(opts)?
    };

    if let Some(save) = &opts.save {
        std::fs::write(save, serde_json::to_string_pretty(&artifact)?)?;
    }

    let report = verify_artifact(&artifact)?;
    print_report(opts, &artifact, &report)?;
    if !report.ok {
        anyhow::bail!("independent replay failed");
    }
    Ok(())
}

fn verify_artifact(artifact: &ReplayArtifact) -> Result<ReplayReport> {
    #[cfg(feature = "wasm")]
    {
        let mut engine = CliWasmEngine;
        let needs_invoke = artifact.commits.iter().any(|commit| {
            commit
                .body
                .as_array()
                .map(|body| {
                    body.iter().any(|action| {
                        action.get("method").and_then(|m| m.as_str()) == Some("invoke")
                    })
                })
                .unwrap_or(false)
        });
        let engine_ref: Option<&mut dyn InvokeEngine> = if needs_invoke {
            Some(&mut engine)
        } else {
            None
        };
        return verify_replay_artifact(artifact, engine_ref);
    }
    #[cfg(not(feature = "wasm"))]
    {
        verify_replay_artifact(artifact, None)
    }
}

fn artifact_from_local_dir(opts: &Opts) -> Result<ReplayArtifact> {
    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let store = ContractStore::open(&dir)?;
    let config = store.load_config()?;
    let contract_id = opts
        .contract_id
        .clone()
        .unwrap_or(config.contract_id.clone());
    let through = opts
        .through
        .clone()
        .or_else(|| store.get_head().ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("no HEAD; pass --through"))?;

    let mut prefix = Vec::new();
    let mut current = Some(through.clone());
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            anyhow::bail!("cycle in local parent chain at {id}");
        }
        let file: CommitFile = store.load_commit(&id)?;
        current = file.head.parent.clone().filter(|parent| !parent.is_empty());
        prefix.push((id, file));
    }
    prefix.reverse();
    artifact_from_prefix(&contract_id, &through, &prefix)
}

async fn fetch_remote_artifact(opts: &Opts, remote: &str) -> Result<ReplayArtifact> {
    let contract_id = if let Some(id) = &opts.contract_id {
        id.clone()
    } else if let Some(dir) = &opts.dir {
        ContractStore::open(dir)?.load_config()?.contract_id
    } else if let Ok(store) = ContractStore::open(&std::env::current_dir()?) {
        store.load_config()?.contract_id
    } else {
        anyhow::bail!("--contract-id is required when fetching a remote prefix without --dir");
    };

    #[cfg(feature = "p2p")]
    {
        let mut node_config = if let Some(node_dir) = &opts.node_dir {
            let config_path = node_dir.join("config.json");
            if config_path.exists() {
                let config_json = std::fs::read_to_string(&config_path)?;
                let mut config: modality_node::config::Config = serde_json::from_str(&config_json)?;
                config.storage_path = None;
                config.logs_path = None;
                config.data_dir = None;
                config.bootstrappers = Some(vec![]);
                let passfile_path = node_dir.join("node.modal_passfile");
                if passfile_path.exists() {
                    config.passfile_path = Some(passfile_path);
                }
                config
            } else {
                modality_node::config::Config::default()
            }
        } else {
            modality_node::config::Config::default()
        };

        if node_config
            .listeners
            .as_ref()
            .map(|l| l.is_empty())
            .unwrap_or(true)
        {
            node_config.listeners = Some(vec!["/ip4/127.0.0.1/tcp/0/ws".parse()?]);
        }
        node_config.bootstrappers = Some(vec![]);

        let _ = modality_node::logging::init_logging(None, Some(false), None);
        let mut node = Node::from_config(node_config.clone()).await?;
        node.setup(&node_config).await?;

        let mut request_data = json!({ "contract_id": contract_id });
        if let Some(through) = &opts.through {
            request_data["through_commit"] = json!(through);
        }

        let response = request::run(
            &mut node,
            remote.to_string(),
            "/contract/replay".to_string(),
            serde_json::to_string(&request_data)?,
        )
        .await?;
        if !response.ok {
            anyhow::bail!("Failed to fetch replay artifact: {:?}", response.errors);
        }
        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No data in replay response"))?;
        return Ok(serde_json::from_value(data)?);
    }
    #[cfg(not(feature = "p2p"))]
    {
        let _ = remote;
        let _ = contract_id;
        anyhow::bail!(
            "P2P remotes require the `p2p` feature. Rebuild with full features or use --artifact."
        );
    }
}

fn print_report(opts: &Opts, artifact: &ReplayArtifact, report: &ReplayReport) -> Result<()> {
    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": report.ok,
                "contract_id": report.contract_id,
                "through_commit": report.through_commit,
                "prefix_digest": report.prefix_digest,
                "commits_checked": report.commits_checked,
                "wasm_modules": report.wasm_modules,
                "invokes_expanded": report.invokes_expanded,
                "errors": report.errors,
            }))?
        );
        return Ok(());
    }

    if report.ok {
        println!("✓ Independent replay passed");
    } else {
        println!("✗ Independent replay failed");
    }
    println!("  Contract:    {}", artifact.contract_id);
    println!("  Through:     {}", artifact.through_commit);
    println!("  Digest:      {}", artifact.prefix_digest);
    println!("  Commits:     {}", report.commits_checked);
    println!("  WASM:        {}", report.wasm_modules);
    if report.invokes_expanded > 0 {
        println!("  Invokes:     {}", report.invokes_expanded);
    }
    for err in &report.errors {
        println!("  Error: {err}");
    }
    Ok(())
}

#[cfg(feature = "wasm")]
pub(crate) struct CliWasmEngine;

#[cfg(all(feature = "wasm", feature = "model-status"))]
pub(crate) fn prefix_from_store(store: &ContractStore) -> Result<Vec<(String, CommitFile)>> {
    let through = store
        .get_head()?
        .ok_or_else(|| anyhow::anyhow!("contract has no HEAD"))?;
    let mut prefix = Vec::new();
    let mut current = Some(through);
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            anyhow::bail!("cycle in local parent chain at {id}");
        }
        let file: CommitFile = store.load_commit(&id)?;
        current = file.head.parent.clone().filter(|parent| !parent.is_empty());
        prefix.push((id, file));
    }
    prefix.reverse();
    Ok(prefix)
}

#[cfg(feature = "wasm")]
impl InvokeEngine for CliWasmEngine {
    fn execute_invoke(
        &mut self,
        wasm: &ReplayWasm,
        args: &Value,
        ctx: &FrozenInvokeContext,
    ) -> Result<Vec<CommitAction>> {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &wasm.bytes_b64)
                .map_err(|err| anyhow::anyhow!("invalid WASM bytes for {}: {err}", wasm.path))?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        if sha256 != wasm.sha256 {
            anyhow::bail!(
                "WASM hash mismatch for {}: artifact {}, computed {}",
                wasm.path,
                wasm.sha256,
                sha256
            );
        }
        let context = ProgramContext {
            contract_id: ctx.contract_id.clone(),
            block_height: ctx.block_height,
            timestamp: FROZEN_INVOKE_TIMESTAMP,
            invoker: ctx.invoker.clone(),
            commit_id: ctx.commit_id.clone(),
            parent_commit_id: ctx.parent_commit_id.clone(),
            state: Value::Object(ctx.state.clone()),
            accepted_state_oracle_keys: ctx.accepted_state_oracle_keys.clone(),
        };
        let input_json = encode_program_input(args.clone(), context)?;
        let mut executor = WasmExecutor::new(wasm.gas_limit);
        let result_json = executor.execute(&bytes, "execute", &input_json)?;
        let result = decode_program_result(&result_json)?;
        validate_program_result(&result)?;
        if !result.is_success() {
            anyhow::bail!("Program execution failed: {:?}", result.errors);
        }
        Ok(result
            .actions
            .into_iter()
            .map(|action| CommitAction {
                method: action.method,
                path: action.path,
                value: action.value,
                source_contract: None,
                source_path: None,
                source_commit: None,
            })
            .collect())
    }
}
