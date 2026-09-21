use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;

use modality_common::contract_store::{CommitFile, ContractStore};
use modality_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Add a commit to a local contract")]
pub struct Opts {
    /// Path in the contract (e.g., /data or /settings/rate)
    #[clap(long)]
    path: Option<String>,

    /// Value to post (can be string, number, or JSON)
    #[clap(long)]
    value: Option<String>,

    /// Method (default: post)
    #[clap(long, default_value = "post")]
    method: String,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,

    // CREATE action fields
    /// Asset ID to create (for CREATE method)
    #[clap(long)]
    asset_id: Option<String>,

    /// Asset quantity (for CREATE method)
    #[clap(long)]
    quantity: Option<u64>,

    /// Asset divisibility (for CREATE method)
    #[clap(long)]
    divisibility: Option<u64>,

    // SEND action fields
    /// Destination contract ID (for SEND method)
    #[clap(long)]
    to_contract: Option<String>,

    /// Amount to send (for SEND method)
    #[clap(long)]
    amount: Option<u64>,

    // RECV action fields
    /// SEND commit ID to receive from (for RECV method)
    #[clap(long)]
    send_commit_id: Option<String>,

    // Signing
    /// Passfile path or identity name for signing the commit; repeat to attach multiple signatures
    #[clap(long)]
    sign: Vec<String>,

    /// Commit all changes from state directory
    #[clap(short = 'a', long)]
    all: bool,

    /// Commit message (optional, stored in commit)
    #[clap(short = 'm', long)]
    message: Option<String>,

    /// ACTION commit (JSON or path to JSON file)
    /// Format: {"method":"ACTION","action":"DO_THING","data":{...}}
    #[clap(long)]
    action: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine contract directory
    let dir = if let Some(d) = &opts.dir {
        d.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&dir)?;
    let config = store.load_config()?;

    // Get current HEAD
    let parent_id = store.get_head()?;

    // Create new commit
    let mut commit = if let Some(parent) = &parent_id {
        CommitFile::with_parent(parent.clone())
    } else {
        CommitFile::new()
    };
    commit.head.message = opts.message.clone();
    let mut committed_repost_dests: Vec<String> = Vec::new();

    // Handle --all flag: commit all changes from state + rules directories
    if opts.all {
        let committed = store.build_state_from_commits()?;
        let state_files = store.list_state_files()?;
        let rules_files = store.list_rules_files()?;
        let accepted_model = accepted_model_content(&store)?;
        let pending_reposts = store.load_pending_reposts()?;

        let mut changes = 0;

        // Add/modify state files (skip dests staged as REPOST)
        for path in &state_files {
            if pending_reposts.contains_key(path) {
                continue;
            }
            if let Some(current_value) = store.read_state(path)? {
                let is_new = !committed.contains_key(path);
                let is_modified = committed
                    .get(path)
                    .map(|v| v != &current_value)
                    .unwrap_or(false);

                if is_new || is_modified {
                    commit.add_action("post".to_string(), Some(path.clone()), current_value);
                    changes += 1;
                }
            }
        }

        for path in store.list_repost_files()? {
            if pending_reposts.contains_key(&path) {
                continue;
            }
            if let Some(current_value) = store.read_working_path(&path)? {
                if committed.get(&path) != Some(&current_value) {
                    anyhow::bail!(
                        "Changed repost {path} has no provenance. Run `modal repost` to refresh it."
                    );
                }
            }
        }

        for (dest_path, provenance) in &pending_reposts {
            let current_value = store.read_working_path(dest_path)?.ok_or_else(|| {
                anyhow::anyhow!("Staged REPOST dest {dest_path} is missing from the working tree")
            })?;
            let is_new = !committed.contains_key(dest_path);
            let is_modified = committed
                .get(dest_path)
                .map(|v| v != &current_value)
                .unwrap_or(false);
            if is_new || is_modified {
                commit.add_repost(
                    dest_path.clone(),
                    current_value,
                    provenance.source_contract.clone(),
                    provenance.source_path.clone(),
                    provenance.source_commit.clone(),
                );
                changes += 1;
            }
            committed_repost_dests.push(dest_path.clone());
        }

        // Add/modify rule files
        for path in &rules_files {
            if let Some(current_value) = store.read_rule(path)? {
                let is_new = !committed.contains_key(path);
                let is_modified = committed
                    .get(path)
                    .map(|v| v != &current_value)
                    .unwrap_or(false);

                if is_new || is_modified {
                    commit.add_action("rule".to_string(), Some(path.clone()), current_value);
                    changes += 1;
                }
            }
        }

        let model_path = dir.join("model").join("default.modality");
        if model_path.exists() {
            let model_content = std::fs::read_to_string(&model_path)?;
            if accepted_model.as_deref() != Some(model_content.as_str()) {
                commit.add_action(
                    "model".to_string(),
                    Some("/model/default.modality".to_string()),
                    Value::String(model_content),
                );
                changes += 1;
            }
        }

        if changes == 0 {
            store.clear_pending_reposts(&committed_repost_dests)?;
            println!("Nothing to commit (working directories match committed state).");
            return Ok(());
        }
    } else if let Some(action_input) = &opts.action {
        // ACTION commit from JSON
        let action_json: Value = if action_input.ends_with(".json") {
            // Load from file
            let content = std::fs::read_to_string(action_input)?;
            serde_json::from_str(&content)?
        } else {
            // Parse as inline JSON
            serde_json::from_str(action_input)?
        };

        // Extract fields from action JSON
        let method = action_json
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("ACTION")
            .to_string();

        let action_name = action_json
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let data = action_json.get("data").cloned();

        // Build the action value
        let value = serde_json::json!({
            "action": action_name,
            "data": data
        });

        commit.add_action(method, opts.path.clone(), value);
    } else if !is_empty_commit(opts) {
        // Single action commit (original behavior)
        let value = match opts.method.as_str() {
            "create" => build_create_value(opts)?,
            "send" => build_send_value(opts)?,
            "recv" => build_recv_value(opts)?,
            "invoke" => build_invoke_value(opts)?,
            _ => {
                // For other methods (post, rule), use the --value flag
                if let Some(value_str) = &opts.value {
                    // Try to parse as JSON, fallback to string
                    serde_json::from_str(value_str)
                        .unwrap_or_else(|_| Value::String(value_str.clone()))
                } else {
                    anyhow::bail!("--value is required for method '{}'", opts.method);
                }
            }
        };

        // Add action
        commit.add_action(opts.method.clone(), opts.path.clone(), value);
    }

    // Sign the commit once per supplied passfile.
    if !opts.sign.is_empty() {
        let mut sig_obj = serde_json::Map::new();
        let body_json = serde_json::to_string(&commit.body)?;

        for passfile_ref in &opts.sign {
            let passfile_path = modality_common::passfile::resolve_passfile_path(passfile_ref)?;
            let passfile_str = passfile_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid passfile path: {}", passfile_path.display())
            })?;
            let keypair = load_signing_key(passfile_str)?;
            let public_key = keypair.public_key_as_base58_identity();
            let signature = keypair.sign_string_as_base64_pad(&body_json)?;
            sig_obj.insert(public_key, Value::String(signature));
        }

        commit.head.signatures = Some(Value::Object(sig_obj));
    }

    // Validate the commit structure
    commit.validate()?;

    // Validate against contract rules (signature predicates, etc.)
    store.validate_commit_against_rules(&commit)?;
    validate_commit_against_model(&dir, &store, &commit)?;

    // Compute commit ID
    let mut commit_id = commit.compute_id()?;

    // Replace $PARENT placeholder in rule values with parent commit ID
    if let Some(parent) = &parent_id {
        for action in &mut commit.body {
            if action.method == "rule" {
                if let Value::String(s) = &action.value {
                    if s.contains("$PARENT") {
                        let replaced = s.replace("$PARENT", parent);

                        // Also update the local rule file so it matches
                        if let Some(path) = &action.path {
                            let _ = store.write_rule(path, &Value::String(replaced.clone()));
                        }

                        action.value = Value::String(replaced);
                    }
                }
            }
        }
        // Recompute commit ID since content changed
        commit_id = commit.compute_id()?;
    }

    // Save commit
    store.save_commit(&commit_id, &commit)?;

    // Update HEAD
    store.set_head(&commit_id)?;
    store.clear_pending_reposts(&committed_repost_dests)?;

    // Output
    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contract_id": config.contract_id,
                "commit_id": commit_id,
                "parent": parent_id,
                "status": "committed",
            }))?
        );
    } else {
        println!("✅ Commit created successfully!");
        println!("   Contract ID: {}", config.contract_id);
        println!("   Commit ID: {}", commit_id);
        if let Some(parent) = parent_id {
            println!("   Parent: {}", parent);
        }
        println!();
        println!("Next steps:");
        println!("  - modal status  (view status)");
        println!("  - modal push    (push to chain)");
    }

    Ok(())
}

fn is_empty_commit(opts: &Opts) -> bool {
    // `modal commit --sign ...` with no path, value, or --all is a signed
    // empty commit: a signature and optional message, no actions.
    opts.path.is_none()
        && opts.value.is_none()
        && opts.action.is_none()
        && !opts.all
        && opts.method.eq_ignore_ascii_case("post")
        && opts.asset_id.is_none()
        && opts.quantity.is_none()
        && opts.divisibility.is_none()
        && opts.to_contract.is_none()
        && opts.amount.is_none()
        && opts.send_commit_id.is_none()
}

fn accepted_model_content(store: &ContractStore) -> Result<Option<String>> {
    let mut current = store.get_head()?;

    while let Some(commit_id) = current {
        let commit = store.load_commit(&commit_id)?;
        if let Some(model_content) = commit.body.iter().rev().find_map(|action| {
            if action.method.eq_ignore_ascii_case("model") {
                action.value.as_str()
            } else {
                None
            }
        }) {
            return Ok(Some(model_content.to_string()));
        }
        current = commit.head.parent.clone();
    }

    Ok(None)
}

#[cfg(feature = "model-status")]
fn validate_commit_against_model(
    dir: &std::path::Path,
    store: &ContractStore,
    commit: &CommitFile,
) -> Result<()> {
    let model_path = dir.join("model").join("default.modality");
    let model_content = if model_path.exists() {
        std::fs::read_to_string(&model_path)?
    } else {
        String::new()
    };

    #[cfg(feature = "wasm")]
    {
        use modality_common::independent_replay::{
            commit_has_invoke, expand_invoke_actions, expand_prefix, frozen_invoke_context,
            wasm_modules_from_commits,
        };
        let prefix = match crate::replay::prefix_from_store(store) {
            Ok(prefix) => prefix,
            Err(_) => Vec::new(),
        };
        if commit_has_invoke(commit) || prefix.iter().any(|(_, file)| commit_has_invoke(file)) {
            let history_files: Vec<_> = prefix.iter().map(|(_, file)| file.clone()).collect();
            let mut wasm = wasm_modules_from_commits(&history_files)?;
            for module in wasm_modules_from_commits(&[commit.clone()])? {
                if modality_common::independent_replay::lookup_wasm(&wasm, &module.path).is_none() {
                    wasm.push(module);
                }
            }
            let contract_id = store.load_config()?.contract_id;
            let mut engine = crate::replay::CliWasmEngine;
            let accepted = if prefix.iter().any(|(_, file)| commit_has_invoke(file)) {
                expand_prefix(&contract_id, &prefix, &wasm, Some(&mut engine))?.0
            } else {
                history_files
            };
            let pending = if commit_has_invoke(commit) {
                let ctx = frozen_invoke_context(&contract_id, "pending", commit, &accepted);
                expand_invoke_actions(commit, &wasm, &ctx, &mut engine)?.0
            } else {
                commit.clone()
            };
            modality_common::model_governance::validate_pending_commit_with_history(
                &model_content,
                &accepted,
                &pending,
            )?;
            return Ok(());
        }
    }

    if model_path.exists() {
        modality_common::model_governance::validate_pending_commit(&model_content, store, commit)?;
    }

    Ok(())
}

#[cfg(not(feature = "model-status"))]
fn validate_commit_against_model(
    _dir: &std::path::Path,
    _store: &ContractStore,
    _commit: &CommitFile,
) -> Result<()> {
    Ok(())
}

fn build_create_value(opts: &Opts) -> Result<Value> {
    let asset_id = opts
        .asset_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--asset-id is required for CREATE method"))?;
    let quantity = opts
        .quantity
        .ok_or_else(|| anyhow::anyhow!("--quantity is required for CREATE method"))?;
    let divisibility = opts
        .divisibility
        .ok_or_else(|| anyhow::anyhow!("--divisibility is required for CREATE method"))?;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "quantity": quantity,
        "divisibility": divisibility
    }))
}

fn build_send_value(opts: &Opts) -> Result<Value> {
    let asset_id = opts
        .asset_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--asset-id is required for SEND method"))?;
    let to_contract = opts
        .to_contract
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--to-contract is required for SEND method"))?;
    let amount = opts
        .amount
        .ok_or_else(|| anyhow::anyhow!("--amount is required for SEND method"))?;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "to_contract": to_contract,
        "amount": amount,
        "identifier": null
    }))
}

fn build_recv_value(opts: &Opts) -> Result<Value> {
    let send_commit_id = opts
        .send_commit_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--send-commit-id is required for RECV method"))?;

    Ok(serde_json::json!({
        "send_commit_id": send_commit_id
    }))
}

fn build_invoke_value(opts: &Opts) -> Result<Value> {
    // For invoke, the value should contain the args
    // The path should point to the program
    if opts.path.is_none() {
        anyhow::bail!("--path is required for INVOKE method (must be /__programs__/{{name}}.wasm)");
    }

    if let Some(value_str) = &opts.value {
        // Parse the value as JSON
        let value: Value = serde_json::from_str(value_str)
            .map_err(|e| anyhow::anyhow!("INVOKE value must be valid JSON: {}", e))?;

        // Ensure it has an args field
        if !value.is_object() || !value.as_object().unwrap().contains_key("args") {
            anyhow::bail!("INVOKE value must be an object with 'args' field");
        }

        Ok(value)
    } else {
        anyhow::bail!("--value is required for INVOKE method (must contain {{\"args\": {{...}}}})");
    }
}

/// Load a signing key from a passfile, prompting for password if encrypted
fn load_signing_key(path: &str) -> anyhow::Result<Keypair> {
    // Try loading as unencrypted first
    let keypair = Keypair::from_json_file(path)?;
    if keypair.can_sign() {
        return Ok(keypair);
    }
    // Has encrypted private key — prompt for password
    eprint!("Password: ");
    let password = rpassword::read_password()
        .map_err(|e| anyhow::anyhow!("Failed to read password: {}", e))?;
    Keypair::from_encrypted_json_file(path, &password)
}
