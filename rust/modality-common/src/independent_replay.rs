//! Independent replay of a sequenced contract prefix.
//!
//! A stranger with this artifact can re-check the log without the original
//! node: sequenced commits, posted WASM (if any), and the same local rule
//! checker used on apply.

use crate::contract_store::{CommitAction, CommitFile};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const REPLAY_ARTIFACT_TYPE: &str = "modality_replay_artifact";
pub const REPLAY_ARTIFACT_VERSION: u32 = 1;
pub const FROZEN_INVOKE_TIMESTAMP: u64 = 0;
pub const DEFAULT_PROGRAM_GAS_LIMIT: u64 = 10_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayArtifact {
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub version: u32,
    pub contract_id: String,
    pub through_commit: String,
    pub prefix_digest: String,
    pub commits: Vec<ReplayCommit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wasm: Vec<ReplayWasm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayCommit {
    pub commit_id: String,
    pub body: Value,
    pub head: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayWasm {
    pub path: String,
    pub sha256: String,
    pub gas_limit: u64,
    pub bytes_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayReport {
    pub ok: bool,
    pub contract_id: String,
    pub through_commit: String,
    pub prefix_digest: String,
    pub commits_checked: usize,
    pub wasm_modules: usize,
    pub invokes_expanded: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrozenInvokeContext {
    pub contract_id: String,
    pub commit_id: String,
    pub parent_commit_id: Option<String>,
    /// Sequenced prefix length before this commit. Not a wall clock and not
    /// a hidden replica height.
    pub block_height: u64,
    /// Frozen at [`FROZEN_INVOKE_TIMESTAMP`]. Programs that need time read
    /// posted contract state.
    pub timestamp: u64,
    /// Lexicographically first signature public key, or empty if unsigned.
    pub invoker: String,
    /// Accepted contract state (paths with a leading `/`).
    pub state: Map<String, Value>,
    /// Accepted-state oracle public keys, keyed by `/oracles/**/*.id` paths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub accepted_state_oracle_keys: BTreeMap<String, String>,
}

pub trait InvokeEngine {
    fn execute_invoke(
        &mut self,
        wasm: &ReplayWasm,
        args: &Value,
        ctx: &FrozenInvokeContext,
    ) -> Result<Vec<CommitAction>>;
}

pub fn prefix_digest(commit_ids: &[String]) -> String {
    let mut hasher = Sha256::new();
    for id in commit_ids {
        hasher.update(id.as_bytes());
        hasher.update([0u8]);
    }
    hex::encode(hasher.finalize())
}

pub fn parse_commit_file(commit_data: &str) -> Result<CommitFile> {
    let mut value: Value = serde_json::from_str(commit_data)?;
    parse_commit_value(&mut value)
}

pub fn parse_commit_value(value: &mut Value) -> Result<CommitFile> {
    if value.get("head").map(|head| head.is_null()).unwrap_or(true) {
        value["head"] = serde_json::json!({});
    }
    if !value
        .get("body")
        .map(|body| body.is_array())
        .unwrap_or(false)
    {
        anyhow::bail!("Invalid commit structure");
    }
    Ok(serde_json::from_value(value.clone())?)
}

pub fn commit_file_from_replay(commit: &ReplayCommit) -> Result<CommitFile> {
    let mut value = serde_json::json!({
        "body": commit.body,
        "head": commit.head,
    });
    parse_commit_value(&mut value)
}

pub fn first_invoker(commit: &CommitFile) -> String {
    let mut keys: Vec<String> = commit
        .head
        .signatures
        .as_ref()
        .and_then(Value::as_object)
        .map(|signatures| signatures.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys.into_iter().next().unwrap_or_default()
}

pub fn accepted_state_from_commits(commits: &[CommitFile]) -> Map<String, Value> {
    let mut state = Map::new();
    for commit in commits {
        apply_commit_to_host_state(commit, &mut state);
    }
    state
}

pub fn accepted_state_oracle_keys_from_state(
    state: &Map<String, Value>,
) -> BTreeMap<String, String> {
    state
        .iter()
        .filter_map(|(path, value)| {
            if path.starts_with("/oracles/") && path.ends_with(".id") {
                value.as_str().map(|key| (path.clone(), key.to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn accepted_state_oracle_keys_from_commits(commits: &[CommitFile]) -> BTreeMap<String, String> {
    accepted_state_oracle_keys_from_state(&accepted_state_from_commits(commits))
}

fn apply_commit_to_host_state(commit: &CommitFile, state: &mut Map<String, Value>) {
    for action in &commit.body {
        let Some(path) = &action.path else {
            continue;
        };
        let path = host_path(path);
        match action.method.to_ascii_lowercase().as_str() {
            "post" | "genesis" | "repost" => {
                state.insert(path, action.value.clone());
            }
            "delete" => {
                state.remove(&path);
            }
            _ => {}
        }
    }
}

pub fn host_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

pub fn frozen_invoke_context(
    contract_id: &str,
    commit_id: &str,
    pending: &CommitFile,
    accepted: &[CommitFile],
) -> FrozenInvokeContext {
    let state = accepted_state_from_commits(accepted);
    let accepted_state_oracle_keys = accepted_state_oracle_keys_from_state(&state);
    FrozenInvokeContext {
        contract_id: contract_id.to_string(),
        commit_id: commit_id.to_string(),
        parent_commit_id: pending.head.parent.clone(),
        block_height: accepted.len() as u64,
        timestamp: FROZEN_INVOKE_TIMESTAMP,
        invoker: first_invoker(pending),
        state,
        accepted_state_oracle_keys,
    }
}

pub fn wasm_modules_from_commits(commits: &[CommitFile]) -> Result<Vec<ReplayWasm>> {
    let mut by_path: HashMap<String, ReplayWasm> = HashMap::new();
    for commit in commits {
        for action in &commit.body {
            if !action.method.eq_ignore_ascii_case("post")
                && !action.method.eq_ignore_ascii_case("genesis")
            {
                continue;
            }
            let Some(path) = action.path.as_deref() else {
                continue;
            };
            if !path.ends_with(".wasm") {
                continue;
            }
            if let Some(wasm) = replay_wasm_from_post(path, &action.value)? {
                by_path.insert(host_path(path), wasm);
            }
        }
    }
    let mut modules: Vec<_> = by_path.into_values().collect();
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(modules)
}

pub fn replay_wasm_from_post(path: &str, value: &Value) -> Result<Option<ReplayWasm>> {
    let (b64, gas_limit) = if let Some(s) = value.as_str() {
        (s.to_string(), DEFAULT_PROGRAM_GAS_LIMIT)
    } else if value.is_object() {
        let Some(b64) = value
            .get("wasm_bytes")
            .or_else(|| value.get("bytes_b64"))
            .and_then(|v| v.as_str())
        else {
            return Ok(None);
        };
        let gas_limit = value
            .get("gas_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_PROGRAM_GAS_LIMIT);
        (b64.to_string(), gas_limit)
    } else {
        return Ok(None);
    };

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64)
        .map_err(|err| anyhow::anyhow!("invalid base64 WASM at {path}: {err}"))?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    Ok(Some(ReplayWasm {
        path: host_path(path),
        sha256,
        gas_limit,
        bytes_b64: b64,
    }))
}

pub fn lookup_wasm<'a>(modules: &'a [ReplayWasm], program_path: &str) -> Option<&'a ReplayWasm> {
    let wanted = host_path(program_path);
    if let Some(found) = modules.iter().find(|module| module.path == wanted) {
        return Some(found);
    }
    let name = wanted.rsplit('/').next().unwrap_or(wanted.as_str());
    modules.iter().find(|module| {
        module
            .path
            .rsplit('/')
            .next()
            .unwrap_or(module.path.as_str())
            == name
    })
}

pub fn commit_has_invoke(commit: &CommitFile) -> bool {
    commit
        .body
        .iter()
        .any(|action| action.method.eq_ignore_ascii_case("invoke"))
}

/// Expand each invoke in a prefix. History used for later commits is the
/// expanded bodies (emitted actions), not the raw `invoke` placeholders.
pub fn expand_prefix(
    contract_id: &str,
    prefix: &[(String, CommitFile)],
    wasm: &[ReplayWasm],
    mut invoke_engine: Option<&mut dyn InvokeEngine>,
) -> Result<(Vec<CommitFile>, usize)> {
    let mut accepted = Vec::new();
    let mut invokes_expanded = 0usize;
    for (id, file) in prefix {
        let pending = if commit_has_invoke(file) {
            let ctx = frozen_invoke_context(contract_id, id, file, &accepted);
            let engine = invoke_engine
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("commit {id}: INVOKE with no WASM executor"))?;
            let (expanded, count) = expand_invoke_actions(file, wasm, &ctx, &mut **engine)?;
            invokes_expanded += count;
            expanded
        } else {
            file.clone()
        };
        accepted.push(pending);
    }
    Ok((accepted, invokes_expanded))
}

/// Expand each invoke in a prefix and re-check with the sequenced rule checker.
#[cfg(feature = "model-governance")]
pub fn expand_and_validate_prefix(
    contract_id: &str,
    prefix: &[(String, CommitFile)],
    wasm: &[ReplayWasm],
    invoke_engine: Option<&mut dyn InvokeEngine>,
) -> Result<(Vec<CommitFile>, usize)> {
    let (expanded, invokes_expanded) = expand_prefix(contract_id, prefix, wasm, invoke_engine)?;
    let mut accepted = Vec::new();
    for (index, pending) in expanded.iter().enumerate() {
        crate::model_governance::validate_sequenced_commit(&accepted, pending)
            .map_err(|err| anyhow::anyhow!("commit {}: {err}", prefix[index].0))?;
        accepted.push(pending.clone());
    }
    Ok((accepted, invokes_expanded))
}

pub fn expand_invoke_actions(
    pending: &CommitFile,
    wasm: &[ReplayWasm],
    ctx: &FrozenInvokeContext,
    engine: &mut dyn InvokeEngine,
) -> Result<(CommitFile, usize)> {
    if !commit_has_invoke(pending) {
        return Ok((pending.clone(), 0));
    }
    let mut expanded = pending.clone();
    let mut new_body = Vec::new();
    let mut expanded_count = 0usize;
    for action in &pending.body {
        if !action.method.eq_ignore_ascii_case("invoke") {
            new_body.push(action.clone());
            continue;
        }
        let path = action
            .path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("INVOKE action missing path"))?;
        let args = action
            .value
            .get("args")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("INVOKE value missing 'args'"))?;
        let module = lookup_wasm(wasm, path).ok_or_else(|| {
            anyhow::anyhow!("INVOKE program {path} is not in the sequenced prefix")
        })?;
        let emitted = engine.execute_invoke(module, &args, ctx)?;
        expanded_count += 1;
        new_body.extend(emitted);
    }
    expanded.body = new_body;
    Ok((expanded, expanded_count))
}

pub fn walk_prefix_ids(
    by_id: &HashMap<String, String>,
    parent_of: &HashMap<String, Option<String>>,
    through_commit: &str,
) -> Result<Vec<String>> {
    if !by_id.contains_key(through_commit) {
        anyhow::bail!("through_commit '{through_commit}' not found on contract");
    }
    let mut walked = Vec::new();
    let mut current = Some(through_commit.to_string());
    let mut seen = HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            anyhow::bail!("cycle in sequenced parent chain at {id}");
        }
        walked.push(id.clone());
        current = parent_of.get(&id).cloned().flatten();
        if current.as_ref() == Some(&id) {
            anyhow::bail!("commit {id} parents itself");
        }
        if walked.len() > 10_000 {
            anyhow::bail!("commit parent chain too long or cyclic");
        }
    }
    walked.reverse();
    Ok(walked)
}

pub fn parent_of_commit_file(commit: &CommitFile) -> Option<String> {
    commit
        .head
        .parent
        .as_ref()
        .filter(|parent| !parent.is_empty())
        .cloned()
}

pub fn sequenced_tips(commits: &[(String, CommitFile)]) -> Vec<String> {
    let mut ids: HashSet<String> = commits.iter().map(|(id, _)| id.clone()).collect();
    for (_id, commit) in commits {
        if let Some(parent) = parent_of_commit_file(commit) {
            ids.remove(&parent);
        }
    }
    let mut tips: Vec<_> = ids.into_iter().collect();
    tips.sort();
    tips
}

pub fn artifact_from_prefix(
    contract_id: &str,
    through_commit: &str,
    prefix: &[(String, CommitFile)],
) -> Result<ReplayArtifact> {
    let ids: Vec<String> = prefix.iter().map(|(id, _)| id.clone()).collect();
    if ids.last().map(String::as_str) != Some(through_commit) {
        anyhow::bail!(
            "prefix does not end at through_commit {through_commit} (last is {:?})",
            ids.last()
        );
    }
    let files: Vec<CommitFile> = prefix.iter().map(|(_, file)| file.clone()).collect();
    let commits = prefix
        .iter()
        .map(|(id, file)| ReplayCommit {
            commit_id: id.clone(),
            body: serde_json::to_value(&file.body).unwrap_or(Value::Array(vec![])),
            head: serde_json::to_value(&file.head).unwrap_or(Value::Object(Map::new())),
        })
        .collect();
    Ok(ReplayArtifact {
        artifact_type: REPLAY_ARTIFACT_TYPE.to_string(),
        version: REPLAY_ARTIFACT_VERSION,
        contract_id: contract_id.to_string(),
        through_commit: through_commit.to_string(),
        prefix_digest: prefix_digest(&ids),
        wasm: wasm_modules_from_commits(&files)?,
        commits,
    })
}

pub fn load_artifact_json(json: &str) -> Result<ReplayArtifact> {
    let artifact: ReplayArtifact = serde_json::from_str(json)?;
    if artifact.artifact_type != REPLAY_ARTIFACT_TYPE {
        anyhow::bail!(
            "not a replay artifact (type is '{}')",
            artifact.artifact_type
        );
    }
    if artifact.version != REPLAY_ARTIFACT_VERSION {
        anyhow::bail!(
            "unsupported replay artifact version {} (expected {})",
            artifact.version,
            REPLAY_ARTIFACT_VERSION
        );
    }
    Ok(artifact)
}

fn report_with_errors(artifact: &ReplayArtifact, errors: Vec<String>) -> ReplayReport {
    ReplayReport {
        ok: errors.is_empty(),
        contract_id: artifact.contract_id.clone(),
        through_commit: artifact.through_commit.clone(),
        prefix_digest: artifact.prefix_digest.clone(),
        commits_checked: 0,
        wasm_modules: artifact.wasm.len(),
        invokes_expanded: 0,
        errors,
    }
}

/// Re-check a fetched prefix. Invoke expansion is required when the log
/// contains `invoke` actions.
pub fn verify_replay_artifact(
    artifact: &ReplayArtifact,
    invoke_engine: Option<&mut dyn InvokeEngine>,
) -> Result<ReplayReport> {
    let mut errors = Vec::new();
    if artifact.commits.is_empty() {
        errors.push("artifact has no commits".to_string());
        return Ok(report_with_errors(artifact, errors));
    }

    let mut files = Vec::new();
    for commit in &artifact.commits {
        match commit_file_from_replay(commit) {
            Ok(file) => files.push((commit.commit_id.clone(), file)),
            Err(err) => errors.push(format!("commit {}: {err}", commit.commit_id)),
        }
    }
    if !errors.is_empty() {
        return Ok(report_with_errors(artifact, errors));
    }

    let ids: Vec<String> = files.iter().map(|(id, _)| id.clone()).collect();
    let expected_digest = prefix_digest(&ids);
    if expected_digest != artifact.prefix_digest {
        errors.push(format!(
            "prefix_digest mismatch: artifact has {}, recomputed {}",
            artifact.prefix_digest, expected_digest
        ));
    }
    if ids.last().map(String::as_str) != Some(artifact.through_commit.as_str()) {
        errors.push(format!(
            "artifact prefix does not end at through_commit {}",
            artifact.through_commit
        ));
    }
    for (index, (id, file)) in files.iter().enumerate() {
        let parent = parent_of_commit_file(file);
        if index == 0 {
            continue;
        }
        let expected_parent = &files[index - 1].0;
        if parent.as_deref() != Some(expected_parent.as_str()) {
            errors.push(format!(
                "commit {id} parent is {parent:?}, expected {expected_parent}"
            ));
        }
    }
    if !errors.is_empty() {
        return Ok(report_with_errors(artifact, errors));
    }

    let needs_invoke = files.iter().any(|(_, file)| commit_has_invoke(file));
    if needs_invoke && invoke_engine.is_none() {
        errors.push(
            "prefix contains INVOKE; rebuild with WASM support to re-execute programs".to_string(),
        );
        return Ok(report_with_errors(artifact, errors));
    }

    let mut wasm = artifact.wasm.clone();
    let extracted = wasm_modules_from_commits(
        &files
            .iter()
            .map(|(_, file)| file.clone())
            .collect::<Vec<_>>(),
    )?;
    for module in extracted {
        if lookup_wasm(&wasm, &module.path).is_none() {
            wasm.push(module);
        }
    }

    let mut invokes_expanded = 0usize;
    let mut accepted: Vec<CommitFile> = Vec::new();
    #[cfg(feature = "model-governance")]
    {
        match expand_and_validate_prefix(&artifact.contract_id, &files, &wasm, invoke_engine) {
            Ok((expanded, count)) => {
                invokes_expanded = count;
                accepted = expanded;
            }
            Err(err) => errors.push(err.to_string()),
        }
    }
    #[cfg(not(feature = "model-governance"))]
    {
        let _ = invoke_engine;
        let _ = wasm;
        let _ = &mut accepted;
        let _ = &mut invokes_expanded;
        errors.push(
            "rule replay requires the model-governance feature; artifact structure checked only"
                .to_string(),
        );
    }

    Ok(ReplayReport {
        ok: errors.is_empty(),
        contract_id: artifact.contract_id.clone(),
        through_commit: artifact.through_commit.clone(),
        prefix_digest: artifact.prefix_digest.clone(),
        commits_checked: if errors.is_empty() {
            files.len()
        } else {
            accepted.len()
        },
        wasm_modules: artifact.wasm.len(),
        invokes_expanded,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(path: &str, value: &str) -> CommitAction {
        CommitAction {
            method: "post".to_string(),
            path: Some(path.to_string()),
            value: Value::String(value.to_string()),
            source_contract: None,
            source_path: None,
            source_commit: None,
        }
    }

    fn file(parent: Option<&str>, body: Vec<CommitAction>) -> CommitFile {
        let mut commit = if let Some(parent) = parent {
            CommitFile::with_parent(parent.to_string())
        } else {
            CommitFile::new()
        };
        commit.body = body;
        commit
    }

    #[test]
    fn prefix_digest_is_order_sensitive() {
        let a = prefix_digest(&["one".into(), "two".into()]);
        let b = prefix_digest(&["two".into(), "one".into()]);
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[test]
    fn frozen_context_has_no_wall_clock() {
        let accepted = vec![file(None, vec![post("/parties/alice.id", "alice_key")])];
        let mut pending = file(Some("g"), vec![post("/notes/x.text", "hi")]);
        pending.head.signatures = Some(serde_json::json!({ "alice_key": "sig" }));
        let ctx = frozen_invoke_context("c1", "c2", &pending, &accepted);
        assert_eq!(ctx.timestamp, 0);
        assert_eq!(ctx.block_height, 1);
        assert_eq!(ctx.invoker, "alice_key");
        assert_eq!(
            ctx.state.get("/parties/alice.id"),
            Some(&Value::String("alice_key".into()))
        );
    }

    #[test]
    fn accepted_state_oracle_keys_are_derived_from_replayed_state() {
        let accepted = vec![file(
            None,
            vec![
                post("/oracles/delivery.id", "delivery_key"),
                post("/oracles/weather/feed.id", "weather_key"),
                post("/parties/alice.id", "alice_key"),
                post("/oracles/not-key.text", "ignored"),
                CommitAction {
                    method: "post".to_string(),
                    path: Some("/oracles/object.id".to_string()),
                    value: serde_json::json!({ "key": "not-a-string" }),
                    source_contract: None,
                    source_path: None,
                    source_commit: None,
                },
            ],
        )];

        let keys = accepted_state_oracle_keys_from_commits(&accepted);

        assert_eq!(keys.len(), 2);
        assert_eq!(
            keys.get("/oracles/delivery.id").map(String::as_str),
            Some("delivery_key")
        );
        assert_eq!(
            keys.get("/oracles/weather/feed.id").map(String::as_str),
            Some("weather_key")
        );
        assert!(!keys.contains_key("/parties/alice.id"));
        assert!(!keys.contains_key("/oracles/object.id"));
    }

    #[test]
    fn frozen_context_carries_replayed_oracle_key_map() {
        let accepted = vec![
            file(None, vec![post("/oracles/delivery.id", "old_key")]),
            file(None, vec![post("/oracles/delivery.id", "new_key")]),
            file(
                None,
                vec![CommitAction {
                    method: "delete".to_string(),
                    path: Some("/oracles/retired.id".to_string()),
                    value: Value::Null,
                    source_contract: None,
                    source_path: None,
                    source_commit: None,
                }],
            ),
        ];
        let pending = file(Some("g"), vec![post("/notes/x.text", "hi")]);
        let ctx = frozen_invoke_context("c1", "c2", &pending, &accepted);

        assert_eq!(
            ctx.accepted_state_oracle_keys
                .get("/oracles/delivery.id")
                .map(String::as_str),
            Some("new_key")
        );
        assert!(!ctx
            .accepted_state_oracle_keys
            .contains_key("/oracles/retired.id"));
    }

    #[test]
    fn extract_wasm_from_posted_base64() {
        let bytes = b"\x00asm not-real";
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);
        let commits = vec![file(
            None,
            vec![CommitAction {
                method: "post".to_string(),
                path: Some("/__programs__/gate.wasm".to_string()),
                value: Value::String(b64.clone()),
                source_contract: None,
                source_path: None,
                source_commit: None,
            }],
        )];
        let modules = wasm_modules_from_commits(&commits).unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].path, "/__programs__/gate.wasm");
        assert_eq!(modules[0].bytes_b64, b64);
        assert!(lookup_wasm(&modules, "/__programs__/gate.wasm").is_some());
        assert!(lookup_wasm(&modules, "gate.wasm").is_some());
    }

    struct EchoEngine;

    impl InvokeEngine for EchoEngine {
        fn execute_invoke(
            &mut self,
            _wasm: &ReplayWasm,
            args: &Value,
            _ctx: &FrozenInvokeContext,
        ) -> Result<Vec<CommitAction>> {
            Ok(vec![post(
                "/notes/from-program.text",
                args.get("msg").and_then(|v| v.as_str()).unwrap_or("x"),
            )])
        }
    }

    #[test]
    fn expand_replaces_invoke_with_emitted_actions() {
        let wasm = ReplayWasm {
            path: "/__programs__/gate.wasm".to_string(),
            sha256: "abc".to_string(),
            gas_limit: 1,
            bytes_b64: "AA==".to_string(),
        };
        let mut pending = file(
            Some("g"),
            vec![CommitAction {
                method: "invoke".to_string(),
                path: Some("/__programs__/gate.wasm".to_string()),
                value: serde_json::json!({"args": {"msg": "hi"}}),
                source_contract: None,
                source_path: None,
                source_commit: None,
            }],
        );
        pending.head.signatures = Some(serde_json::json!({"alice_key": "sig"}));
        let ctx = frozen_invoke_context("c1", "inv", &pending, &[]);
        let (expanded, count) =
            expand_invoke_actions(&pending, &[wasm], &ctx, &mut EchoEngine).unwrap();
        assert_eq!(count, 1);
        assert_eq!(expanded.body.len(), 1);
        assert_eq!(expanded.body[0].method, "post");
        assert_eq!(
            expanded.body[0].path.as_deref(),
            Some("/notes/from-program.text")
        );
        assert_eq!(first_invoker(&expanded), "alice_key");
    }

    #[cfg(feature = "model-governance")]
    #[test]
    fn replay_rejects_unsigned_post_after_posted_model() {
        let model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
"#;
        let genesis = file(
            None,
            vec![
                post("/parties/alice.id", "alice_key"),
                CommitAction {
                    method: "model".to_string(),
                    path: Some("/model/default.modality".to_string()),
                    value: Value::String(model.to_string()),
                    source_contract: None,
                    source_path: None,
                    source_commit: None,
                },
            ],
        );
        let unsigned = file(Some("g"), vec![post("/notes/nope.text", "nope")]);
        let artifact = artifact_from_prefix(
            "c1",
            "bad",
            &[("g".into(), genesis), ("bad".into(), unsigned)],
        )
        .unwrap();
        let report = verify_replay_artifact(&artifact, None).unwrap();
        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|err| err.contains("missing +signed_by(/parties/alice.id)")));
    }

    #[cfg(feature = "model-governance")]
    #[test]
    fn replay_accepts_signed_post_after_posted_model() {
        let model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
"#;
        let genesis = file(
            None,
            vec![
                post("/parties/alice.id", "alice_key"),
                CommitAction {
                    method: "model".to_string(),
                    path: Some("/model/default.modality".to_string()),
                    value: Value::String(model.to_string()),
                    source_contract: None,
                    source_path: None,
                    source_commit: None,
                },
            ],
        );
        let mut signed = file(Some("g"), vec![post("/notes/ok.text", "yes")]);
        signed.head.signatures = Some(serde_json::json!({"alice_key": "sig"}));
        let artifact =
            artifact_from_prefix("c1", "ok", &[("g".into(), genesis), ("ok".into(), signed)])
                .unwrap();
        let report = verify_replay_artifact(&artifact, None).unwrap();
        assert!(report.ok, "{:?}", report.errors);
        assert_eq!(report.commits_checked, 2);
    }
}
