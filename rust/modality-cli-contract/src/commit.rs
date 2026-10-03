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

    /// Decimal places a wallet shows for the asset, for CREATE (display
    /// only: 8 shows 150000000 as 1.5)
    #[clap(long)]
    decimals: Option<u32>,

    // SEND action fields
    /// Destination contract ID (for SEND method)
    #[clap(long, value_parser = modality_common::peer_id::peer_id_arg)]
    to_contract: Option<String>,

    /// Amount to send (for SEND method). With RECV, the amount the SEND must move
    #[clap(long)]
    amount: Option<u64>,

    /// Contract that created the asset, when it is not this contract (for SEND,
    /// and for RECV as the creator the SEND must move)
    #[clap(long, value_parser = modality_common::peer_id::peer_id_arg)]
    asset_contract: Option<String>,

    /// JSON the SEND carries for the recipient (for SEND method)
    #[clap(long)]
    memo: Option<String>,

    // RECV action fields
    /// SEND commit ID to receive from (for RECV method)
    #[clap(long)]
    send_commit_id: Option<String>,

    /// Which SEND of that commit, counting from 0 after any invoke is
    /// expanded (for RECV method; default 0)
    #[clap(long)]
    send_index: Option<u64>,

    // Signing
    /// Passfile path or identity name for signing the commit; repeat to attach multiple signatures
    #[clap(long)]
    sign: Vec<String>,

    /// Commit all changes from state directory. With --method create, send,
    /// recv or invoke, that action joins the same commit
    #[clap(short = 'a', long)]
    all: bool,

    /// Commit message (optional, stored in commit)
    #[clap(short = 'm', long)]
    message: Option<String>,

    /// ACTION commit (JSON or path to JSON file)
    /// Format: {"method":"ACTION","action":"DO_THING","data":{...}}
    #[clap(long)]
    action: Option<String>,

    /// POST a value at a path in this commit, whether or not it changed;
    /// repeat for several paths. The value is read as JSON, else as a
    /// string. Alone, or beside --all or --path
    #[clap(long = "post", value_name = "PATH=VALUE")]
    posts: Vec<String>,

    /// Predicate theory local verify runs: v3, the testnet's. Use v0 for a
    /// network whose network.json leaves predicate_theory_version unset, and
    /// v2 for one that sets it to v2 (numbers compared as 64-bit floats)
    #[clap(long, default_value = "v3", value_parser = ["v0", "v2", "v3"])]
    theory: String,

    /// The most gas this commit may use (signed into its head). Without it,
    /// the network's default limit applies. `modal commit` prints the gas a
    /// commit uses.
    #[clap(long)]
    gas_limit: Option<u64>,

    /// The wallet that pays this commit's gas (Modality ID or passfile).
    /// Its key must also sign (`--sign`). Needed where gas is priced.
    #[clap(long)]
    payer: Option<String>,

    /// A tip per gas (smallest MOD units) for the sequencer that orders the
    /// commit. Rounds fill highest tip first.
    #[clap(long)]
    gas_tip: Option<u64>,

    /// The most the commit will pay per gas, base and tip together.
    #[clap(long)]
    max_gas_price: Option<u64>,
}

/// A commit made by [`make`].
pub struct Committed {
    pub contract_id: String,
    pub commit_id: String,
    pub parent: Option<String>,
    /// The gas the commit uses under schedule v1.
    pub gas: modality_common::gas::GasUsed,
    theory_preview: Option<TheoryPreview>,
}

/// Make the commit `opts` describes, check it, and move HEAD to it, without
/// printing. `None` when there is nothing to commit.
pub async fn make(opts: &Opts) -> Result<Option<Committed>> {
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

        let method = opts.method.to_ascii_lowercase();
        if matches!(method.as_str(), "create" | "send" | "recv" | "invoke") {
            let value = match method.as_str() {
                "create" => build_create_value(opts)?,
                "send" => build_send_value(opts)?,
                "recv" => build_recv_value(opts)?,
                _ => build_invoke_value(opts)?,
            };
            commit.add_action(method, opts.path.clone(), value);
            changes += 1;
        }

        changes += add_posts(&mut commit, &opts.posts)?;

        if changes == 0 {
            store.clear_pending_reposts(&committed_repost_dests)?;
            return Ok(None);
        }
    } else if !opts.posts.is_empty()
        && opts.path.is_none()
        && opts.value.is_none()
        && opts.action.is_none()
        && opts.method.eq_ignore_ascii_case("post")
    {
        add_posts(&mut commit, &opts.posts)?;
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
                    if opts.path.as_deref().is_some_and(|p| p.ends_with(".id")) {
                        Value::String(modality_common::peer_id::id_value(value_str))
                    } else {
                        // Try to parse as JSON, fallback to string
                        serde_json::from_str(value_str)
                            .unwrap_or_else(|_| Value::String(value_str.clone()))
                    }
                } else {
                    anyhow::bail!("--value is required for method '{}'", opts.method);
                }
            }
        };

        // Add action
        commit.add_action(opts.method.clone(), opts.path.clone(), value);
        add_posts(&mut commit, &opts.posts)?;
    }

    if let Some(limit) = opts.gas_limit {
        commit.head.gas_limit = Some(limit);
    }
    if let Some(payer) = &opts.payer {
        commit.head.payer = Some(crate::signer_set::signer_id(payer)?);
    }
    commit.head.gas_tip = opts.gas_tip;
    commit.head.max_gas_price = opts.max_gas_price;

    // Sign the commit once per supplied passfile.
    if !opts.sign.is_empty() {
        let mut sig_obj = serde_json::Map::new();

        for passfile_ref in &opts.sign {
            let passfile_path = modality_common::passfile::resolve_passfile_path(passfile_ref)?;
            let passfile_str = passfile_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid passfile path: {}", passfile_path.display())
            })?;
            let keypair = load_signing_key(passfile_str)?;
            let (public_key, signature) = modality_common::commit_signatures::sign_commit(
                &keypair,
                &config.contract_id,
                &commit,
            )?;
            sig_obj.insert(public_key, Value::String(signature));
        }

        commit.head.signatures = Some(Value::Object(sig_obj));
    }

    // Validate the commit structure
    commit.validate()?;

    // Validate against contract rules (signature predicates, etc.)
    store.validate_commit_against_rules(&commit)?;
    let theory_preview = validate_commit_against_model(&dir, &store, &commit, &opts.theory)?;
    let gas = estimate_gas(&store, &commit)?;

    let commit_id = commit.compute_id()?;

    // Save commit
    store.save_commit(&commit_id, &commit)?;

    // Update HEAD
    store.set_head(&commit_id)?;
    store.clear_pending_reposts(&committed_repost_dests)?;
    if commit.body.iter().any(|a| a.method == "invoke") {
        // The program's writes are state now.
        crate::checkout::checkout(&store)?;
    }

    Ok(Some(Committed {
        gas,
        contract_id: config.contract_id.clone(),
        commit_id,
        parent: parent_id,
        theory_preview,
    }))
}

pub async fn run(opts: &Opts) -> Result<()> {
    let Some(Committed {
        contract_id,
        commit_id,
        parent: parent_id,
        gas,
        theory_preview,
    }) = make(opts).await?
    else {
        println!("Nothing to commit (working directories match committed state).");
        return Ok(());
    };
    let gas_limit_shown = opts.gas_limit;

    // Output
    if opts.output == "json" {
        let mut out = serde_json::json!({
            "contract_id": contract_id,
            "commit_id": commit_id,
            "parent": parent_id,
            "status": "committed",
            "gas": {
                "total": gas.total(),
                "ordering": gas.ordering,
                "apply": gas.apply,
                "fuel": gas.fuel,
                "limit": gas_limit_shown,
            },
        });
        if let Some(preview) = &theory_preview {
            out["theory_preview"] = preview.json.clone();
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("✅ Commit created successfully!");
        println!("   Contract ID: {}", contract_id);
        println!("   Commit ID: {}", commit_id);
        println!(
            "   Gas: {} (ordering {}, apply {}{}){}",
            gas.total(),
            gas.ordering,
            gas.apply,
            if gas.fuel > 0 {
                format!(", fuel {}", gas.fuel)
            } else {
                String::new()
            },
            match gas_limit_shown {
                Some(limit) => format!(", limit {limit}"),
                None => String::new(),
            }
        );
        if let Some(parent) = parent_id {
            println!("   Parent: {}", parent);
        }
        if let Some(preview) = &theory_preview {
            println!();
            println!(
                "⚠️  Predicate theory {} preview (local verify used --theory v0; the testnet, and any network that sets predicate_theory_version v3, enforces this):",
                preview.theory
            );
            for line in &preview.lines {
                println!("   - {line}");
            }
            println!("   Run `modal contract theory` for the full view.");
        }
        println!();
        println!("Next steps:");
        println!("  - modal status  (view status)");
        println!("  - modal push    (push to chain)");
    }

    Ok(())
}

/// `--post PATH=VALUE` actions, in the order given. Returns how many.
fn add_posts(commit: &mut CommitFile, posts: &[String]) -> Result<usize> {
    for post in posts {
        let (path, value) = post
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--post wants PATH=VALUE, got '{post}'"))?;
        if !path.starts_with('/') {
            anyhow::bail!("--post path must start with '/', got '{path}'");
        }
        // Text-typed paths keep the value as written: a sha of digits is
        // still text.
        let textual = [".text", ".md", ".id", ".date"]
            .iter()
            .any(|ext| path.ends_with(ext));
        let value = if path.ends_with(".id") {
            Value::String(modality_common::peer_id::id_value(value))
        } else if textual {
            Value::String(value.to_string())
        } else {
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
        };
        commit.add_action("post".to_string(), Some(path.to_string()), value);
    }
    Ok(posts.len())
}

fn is_empty_commit(opts: &Opts) -> bool {
    // `modal commit --sign ...` with no path, value, or --all is a signed
    // empty commit: a signature and optional message, no actions.
    opts.path.is_none()
        && opts.posts.is_empty()
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
        && opts.send_index.is_none()
        && opts.asset_contract.is_none()
        && opts.memo.is_none()
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

/// What a newer predicate theory version would change about a commit the
/// network accepts today. Shown, never enforced.
#[cfg_attr(not(feature = "model-status"), allow(dead_code))]
#[derive(Debug)]
struct TheoryPreview {
    theory: String,
    json: Value,
    lines: Vec<String>,
}

#[cfg(feature = "model-status")]
fn theory_preview(
    report: modality_common::model_governance::ShadowReport,
) -> Option<TheoryPreview> {
    use modality_common::model_governance::TheoryFinding;

    if report.findings.is_empty() {
        return None;
    }
    let has_dead_edge = report
        .findings
        .iter()
        .any(|f| matches!(f, TheoryFinding::DeadEdge { .. }));
    let mut verdicts = Vec::new();
    let mut details = Vec::new();
    for finding in &report.findings {
        match finding {
            TheoryFinding::WouldRefuse { .. } if has_dead_edge => {
                verdicts.push("this commit would be refused: the model has dead edges".to_string())
            }
            TheoryFinding::WouldRefuse { reason } => {
                verdicts.push(format!("this commit would be refused: {reason}"))
            }
            TheoryFinding::WouldAccept { .. } => {
                verdicts.push("this commit would be accepted".to_string())
            }
            TheoryFinding::DeadEdge {
                part,
                from,
                to,
                offending,
            } => details.push(format!(
                "dead edge {part}: {from} --> {to}; cannot hold together: {}",
                offending.join(", ")
            )),
            TheoryFinding::DeclarationUnparsed { module } => details.push(format!(
                "the declaration for {module} is outside the theory; its predicate stays opaque"
            )),
            TheoryFinding::DeadAfterStep {
                part,
                from,
                to,
                offending,
            } => details.push(format!(
                "warning (modality/dead-end-after-step): {part}: {from} --> {to} is never taken once the contract is under way; cannot hold together: {}",
                offending.join(", ")
            )),
        }
    }
    verdicts.extend(details);
    let lines = verdicts;
    Some(TheoryPreview {
        theory: report.theory.clone(),
        json: serde_json::to_value(&report).ok()?,
        lines,
    })
}

/// The copy's commits, genesis first.
fn history_oldest_first(store: &ContractStore) -> Result<Vec<CommitFile>> {
    let mut commits = Vec::new();
    let mut current = store.get_head()?;
    while let Some(id) = current {
        let commit = store.load_commit(&id)?;
        current = commit.head.parent.clone().filter(|p| !p.is_empty());
        commits.push(commit);
    }
    commits.reverse();
    Ok(commits)
}

/// The gas `commit` uses under schedule v1, as a sequencer will meter it:
/// its programs run here against the copy's history. Refuses a commit over
/// its own `gas_limit`, as the network would.
fn estimate_gas(
    store: &ContractStore,
    commit: &CommitFile,
) -> Result<modality_common::gas::GasUsed> {
    use modality_common::gas::{commit_limit, governing_source, meter, within_limit, SCHEDULE_V1};
    let history = history_oldest_first(store)?;
    #[allow(unused_mut)]
    let mut accepted = history.clone();
    #[allow(unused_mut)]
    let mut expanded = commit.clone();
    #[allow(unused_mut)]
    let mut fuel = 0u64;
    let limit = commit_limit(&SCHEDULE_V1, commit);
    let governing = governing_source(&accepted, commit);
    let before = meter(&SCHEDULE_V1, &governing, commit, commit, 0);
    if commit.head.gas_limit.is_some() {
        within_limit(&before, limit)?;
    }
    #[cfg(all(feature = "wasm", feature = "model-status"))]
    {
        use modality_common::independent_replay::{
            commit_has_invoke, expand_invoke_actions_within, expand_prefix, frozen_invoke_context,
            wasm_modules_from_commits,
        };
        if commit_has_invoke(commit) {
            let prefix = if store.get_head()?.is_some() {
                crate::replay::prefix_from_store(store)?
            } else {
                Vec::new()
            };
            let mut wasm = wasm_modules_from_commits(&history)?;
            for module in wasm_modules_from_commits(std::slice::from_ref(commit))? {
                if modality_common::independent_replay::lookup_wasm(&wasm, &module.path).is_none() {
                    wasm.push(module);
                }
            }
            let contract_id = store.load_config()?.contract_id;
            let mut engine = crate::replay::CliWasmEngine::default();
            if prefix.iter().any(|(_, file)| commit_has_invoke(file)) {
                accepted = expand_prefix(&contract_id, &prefix, &wasm, Some(&mut engine))?.0;
            }
            let ctx = frozen_invoke_context(&contract_id, "pending", commit, &accepted);
            expanded = expand_invoke_actions_within(
                commit,
                &wasm,
                &ctx,
                &mut engine,
                limit.saturating_sub(before.total()),
                &mut fuel,
            )?
            .0;
        }
    }
    let used = meter(&SCHEDULE_V1, &governing, commit, &expanded, fuel);
    if commit.head.gas_limit.is_some() {
        within_limit(&used, limit)?;
    }
    Ok(used)
}

#[cfg(feature = "model-status")]
fn validate_commit_against_model(
    dir: &std::path::Path,
    store: &ContractStore,
    commit: &CommitFile,
    theory: &str,
) -> Result<Option<TheoryPreview>> {
    use modality_common::model_governance::{
        load_commits_oldest_first, shadow_findings_for_store, validate_pending_commit_with_theory,
        TheoryActivation,
    };
    use modality_lang::TheoryVersion;

    let version: TheoryVersion = theory.parse().map_err(anyhow::Error::msg)?;
    // Under v0, local verify previews what v3 (the testnet) would refuse.
    // v3's entailment is v2's; its difference is exact numeric comparison.
    let v2 = version != TheoryVersion::V0;
    let activation = TheoryActivation::always(version);

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
        let prefix = crate::replay::prefix_from_store(store).unwrap_or_default();
        if commit_has_invoke(commit) || prefix.iter().any(|(_, file)| commit_has_invoke(file)) {
            let history_files: Vec<_> = prefix.iter().map(|(_, file)| file.clone()).collect();
            let mut wasm = wasm_modules_from_commits(&history_files)?;
            for module in wasm_modules_from_commits(std::slice::from_ref(commit))? {
                if modality_common::independent_replay::lookup_wasm(&wasm, &module.path).is_none() {
                    wasm.push(module);
                }
            }
            let contract_id = store.load_config()?.contract_id;
            let mut engine = crate::replay::CliWasmEngine::default();
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
            validate_pending_commit_with_theory(
                &model_content,
                &accepted,
                &pending,
                None,
                None,
                None,
                activation,
            )?;
            if v2 {
                return Ok(None);
            }
            return Ok(theory_preview(
                modality_common::model_governance::shadow_findings(
                    &model_content,
                    &accepted,
                    &pending,
                    TheoryVersion::V3,
                ),
            ));
        }
    }

    // The governing model is the accepted one, falling back to the working
    // file. A copy made by `pull` may have no file; its history still binds.
    let accepted = load_commits_oldest_first(store)?;
    validate_pending_commit_with_theory(
        &model_content,
        &accepted,
        commit,
        None,
        None,
        None,
        activation,
    )?;
    if v2 {
        return Ok(None);
    }
    Ok(
        shadow_findings_for_store(&model_content, store, commit, TheoryVersion::V3)
            .ok()
            .and_then(theory_preview),
    )
}

#[cfg(all(test, feature = "model-status"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SLIPPED_ESCROW: &str = r#"export default model {
  start --> open: +MODEL
  open --> open: +post_to_path(/escrow/paid.num)
  open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
  open --> refunded: +signed_by(/parties/bob.id) +num_lt(/escrow/paid.num,"100") +num_gte(/escrow/paid.num,"100")
}
"#;

    #[tokio::test]
    async fn v2_refuses_a_dead_edge_that_v0_only_previews() -> Result<()> {
        let temp = TempDir::new()?;
        let dir = temp.path().join("escrow");
        let dir_arg = dir.to_string_lossy().to_string();
        crate::create::run(&crate::create::Opts::parse_from([
            "create",
            "--dir",
            dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;
        crate::checkout::run(&crate::checkout::Opts::parse_from([
            "checkout",
            "--dir",
            dir_arg.as_str(),
        ]))
        .await?;
        std::fs::write(dir.join("model/default.modality"), SLIPPED_ESCROW)?;

        let store = ContractStore::open(&dir)?;
        let mut commit = CommitFile::with_parent(store.get_head()?.expect("genesis HEAD"));
        commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(SLIPPED_ESCROW.to_string()),
        );
        let preview = validate_commit_against_model(&dir, &store, &commit, "v0")?
            .expect("V2 has something to say about a dead edge");
        let err = validate_commit_against_model(&dir, &store, &commit, "v2")
            .expect_err("--theory v2 refuses the dead edge");
        assert!(err.to_string().contains("open --> refunded"), "{err}");
        assert_eq!(preview.theory, "V3");
        assert!(
            preview.lines[0].contains("would be refused"),
            "{:?}",
            preview.lines
        );
        assert!(
            preview
                .lines
                .iter()
                .any(|l| l.contains("open --> refunded")),
            "{:?}",
            preview.lines
        );

        let err = crate::commit::run(&Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await
        .expect_err("the default theory is v3, which refuses the dead edge");
        assert!(err.to_string().contains("open --> refunded"), "{err}");
        crate::commit::run(&Opts::parse_from([
            "commit",
            "--all",
            "--theory",
            "v0",
            "--dir",
            dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;
        crate::theory::run(&crate::theory::Opts::parse_from([
            "theory",
            "--dir",
            dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;
        Ok(())
    }
}

#[cfg(not(feature = "model-status"))]
fn validate_commit_against_model(
    _dir: &std::path::Path,
    _store: &ContractStore,
    _commit: &CommitFile,
    _theory: &str,
) -> Result<Option<TheoryPreview>> {
    Ok(None)
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

    let mut value = serde_json::json!({
        "asset_id": asset_id,
        "quantity": quantity,
        "divisibility": divisibility
    });
    if let Some(decimals) = opts.decimals {
        value["decimals"] = decimals.into();
    }
    Ok(value)
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

    let mut value = serde_json::json!({
        "asset_id": asset_id,
        "to_contract": to_contract,
        "amount": amount,
        "identifier": null
    });
    if let Some(creator) = &opts.asset_contract {
        value["asset_contract"] = serde_json::json!(creator);
    }
    if let Some(memo) = &opts.memo {
        value["memo"] =
            serde_json::from_str(memo).map_err(|e| anyhow::anyhow!("--memo must be JSON: {e}"))?;
    }
    Ok(value)
}

fn build_recv_value(opts: &Opts) -> Result<Value> {
    let send_commit_id = opts
        .send_commit_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--send-commit-id is required for RECV method"))?;

    let mut value = serde_json::json!({
        "send_commit_id": send_commit_id
    });
    if let Some(index) = opts.send_index {
        value["send_index"] = serde_json::json!(index);
    }
    if let Some(asset_id) = &opts.asset_id {
        value["asset_id"] = serde_json::json!(asset_id);
    }
    if let Some(creator) = &opts.asset_contract {
        value["asset_contract"] = serde_json::json!(creator);
    }
    if let Some(amount) = opts.amount {
        value["amount"] = serde_json::json!(amount);
    }
    Ok(value)
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
