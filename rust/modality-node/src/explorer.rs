//! Local HTTP reads of sequenced contracts for the observer-served explorer.

use anyhow::Result;
use modality_common::independent_replay::{
    parse_commit_file, sequenced_tips, verify_replay_artifact, ReplayArtifact, ReplayReport,
};
use modality_datastore::models::{Commit, Contract};
use modality_datastore::DatastoreManager;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::reqres::contract::replay::export_replay_artifact;

#[derive(Debug, Clone, Serialize)]
pub struct ContractSummary {
    pub contract_id: String,
    pub created_at: Option<u64>,
    pub commits_total: usize,
    pub commits_sequenced: usize,
    pub head: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractInspect {
    pub contract_id: String,
    pub created_at: Option<u64>,
    pub head: Option<String>,
    pub heads: Vec<String>,
    pub sequenced: bool,
    pub commits_total: usize,
    pub commits_sequenced: usize,
    pub state: BTreeMap<String, Value>,
    pub model: BTreeMap<String, Value>,
    pub rules: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitView {
    pub commit_id: String,
    pub timestamp: u64,
    pub parent: Option<String>,
    pub signatures: Value,
    pub in_batch: Option<String>,
    pub sequenced: bool,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayView {
    pub artifact: ReplayArtifact,
    pub report: ReplayReport,
}

pub async fn list_contracts(datastore: &DatastoreManager) -> Result<Vec<ContractSummary>> {
    let mut ids = Commit::list_contract_ids_multi(datastore).await?;
    for contract in Contract::find_all_multi(datastore).await? {
        if !ids.iter().any(|id| id == &contract.contract_id) {
            ids.push(contract.contract_id);
        }
    }
    ids.sort();
    ids.dedup();

    let mut out = Vec::new();
    for contract_id in ids {
        out.push(summarize_contract(datastore, &contract_id).await?);
    }
    Ok(out)
}

pub async fn inspect_contract(
    datastore: &DatastoreManager,
    contract_id: &str,
) -> Result<Option<ContractInspect>> {
    let commits = Commit::find_by_contract_multi(datastore, contract_id).await?;
    if commits.is_empty()
        && Contract::find_by_id_multi(datastore, contract_id)
            .await?
            .is_none()
    {
        return Ok(None);
    }

    let summary = summarize_from_commits(datastore, contract_id, &commits).await?;
    let (state, model, rules) = split_state_tree(&commits);

    Ok(Some(ContractInspect {
        contract_id: contract_id.to_string(),
        created_at: summary.created_at,
        head: summary.head.clone(),
        heads: sequenced_head_ids(&commits),
        sequenced: summary.commits_sequenced > 0,
        commits_total: summary.commits_total,
        commits_sequenced: summary.commits_sequenced,
        state,
        model,
        rules,
    }))
}

pub async fn list_commits(
    datastore: &DatastoreManager,
    contract_id: &str,
) -> Result<Option<Vec<CommitView>>> {
    let commits = Commit::find_by_contract_multi(datastore, contract_id).await?;
    if commits.is_empty()
        && Contract::find_by_id_multi(datastore, contract_id)
            .await?
            .is_none()
    {
        return Ok(None);
    }
    Ok(Some(commits.iter().map(commit_view).collect()))
}

pub async fn replay_contract(
    datastore: &DatastoreManager,
    contract_id: &str,
    through_commit: Option<&str>,
) -> Result<Option<ReplayView>> {
    let commits = Commit::find_by_contract_multi(datastore, contract_id).await?;
    if commits.iter().all(|c| !c.is_sequenced()) {
        if commits.is_empty()
            && Contract::find_by_id_multi(datastore, contract_id)
                .await?
                .is_none()
        {
            return Ok(None);
        }
        anyhow::bail!("no sequenced commits for contract {contract_id}");
    }
    let artifact = export_replay_artifact(datastore, contract_id, through_commit).await?;
    let report = verify_replay_artifact(&artifact, None)?;
    Ok(Some(ReplayView { artifact, report }))
}

async fn summarize_contract(
    datastore: &DatastoreManager,
    contract_id: &str,
) -> Result<ContractSummary> {
    let commits = Commit::find_by_contract_multi(datastore, contract_id).await?;
    summarize_from_commits(datastore, contract_id, &commits).await
}

async fn summarize_from_commits(
    datastore: &DatastoreManager,
    contract_id: &str,
    commits: &[Commit],
) -> Result<ContractSummary> {
    let created_at = Contract::find_by_id_multi(datastore, contract_id)
        .await?
        .map(|c| c.created_at);
    let commits_sequenced = commits.iter().filter(|c| c.is_sequenced()).count();
    Ok(ContractSummary {
        contract_id: contract_id.to_string(),
        created_at,
        commits_total: commits.len(),
        commits_sequenced,
        head: sequenced_head_ids(commits).into_iter().next(),
    })
}

fn sequenced_head_ids(commits: &[Commit]) -> Vec<String> {
    let sequenced: Vec<(String, _)> = commits
        .iter()
        .filter(|c| c.is_sequenced())
        .filter_map(|c| {
            parse_commit_file(&c.commit_data)
                .ok()
                .map(|file| (c.commit_id.clone(), file))
        })
        .collect();
    sequenced_tips(&sequenced)
}

fn commit_view(commit: &Commit) -> CommitView {
    let file = parse_commit_file(&commit.commit_data).ok();
    let parent = file
        .as_ref()
        .and_then(|f| f.head.parent.clone())
        .filter(|p| !p.is_empty());
    let signatures = file
        .as_ref()
        .and_then(|f| f.head.signatures.clone())
        .unwrap_or(Value::Null);
    let methods = file
        .as_ref()
        .map(|f| f.body.iter().map(|a| a.method.clone()).collect())
        .unwrap_or_default();
    CommitView {
        commit_id: commit.commit_id.clone(),
        timestamp: commit.timestamp,
        parent,
        signatures,
        in_batch: commit.in_batch.clone(),
        sequenced: commit.is_sequenced(),
        methods,
    }
}

fn split_state_tree(
    commits: &[Commit],
) -> (
    BTreeMap<String, Value>,
    BTreeMap<String, Value>,
    BTreeMap<String, Value>,
) {
    let mut paths: BTreeMap<String, Value> = BTreeMap::new();
    let mut ordered: Vec<(String, _)> = commits
        .iter()
        .filter(|c| c.is_sequenced())
        .filter_map(|c| {
            parse_commit_file(&c.commit_data)
                .ok()
                .map(|file| (c.commit_id.clone(), file))
        })
        .collect();
    let mut by_id: BTreeMap<String, _> = ordered.iter().cloned().collect();
    let tips = sequenced_tips(&ordered);
    ordered.clear();
    let mut seen = std::collections::HashSet::new();
    if let Some(through) = tips.first() {
        let mut current = Some(through.clone());
        while let Some(id) = current {
            if !seen.insert(id.clone()) {
                break;
            }
            if let Some(file) = by_id.remove(&id) {
                current = file.head.parent.clone().filter(|parent| !parent.is_empty());
                ordered.push((id, file));
            } else {
                break;
            }
        }
        ordered.reverse();
    }
    for (_id, file) in ordered {
        for action in &file.body {
            let Some(path) = action.path.as_ref() else {
                continue;
            };
            match action.method.as_str() {
                "post" | "genesis" | "rule" | "repost" | "model" => {
                    paths.insert(path.clone(), action.value.clone());
                }
                _ => {}
            }
        }
    }

    let mut state = BTreeMap::new();
    let mut model = BTreeMap::new();
    let mut rules = BTreeMap::new();
    for (path, value) in paths {
        if path.starts_with("/model/") {
            model.insert(path, value);
        } else if path.starts_with("/rules/") || path.ends_with(".modality") {
            rules.insert(path, value);
        } else {
            state.insert(path, value);
        }
    }
    (state, model, rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_validator::ContractProcessor;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn seed_note() -> DatastoreManager {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        {
            let processor = ContractProcessor::new(ds.clone());
            processor
                .process_commit(
                    "alice",
                    "g",
                    &json!({
                        "body": [
                            { "method": "post", "path": "/notes/hi.text", "value": "hello" },
                            { "method": "post", "path": "/model/default.modality", "value": "model M { }" },
                            { "method": "post", "path": "/rules/ok.modality", "value": "always" }
                        ],
                        "head": {}
                    })
                    .to_string(),
                )
                .await
                .unwrap();
        }
        {
            let mgr = ds.lock().await;
            let keys = [
                ("contract_id".to_string(), "alice".to_string()),
                ("commit_id".to_string(), "g".to_string()),
            ]
            .into_iter()
            .collect();
            let mut commit = Commit::find_one_multi(&mgr, keys).await.unwrap().unwrap();
            commit.in_batch = Some("batch-1".into());
            commit.save_to_final(&mgr).await.unwrap();
        }
        Arc::try_unwrap(ds)
            .ok()
            .expect("processor dropped")
            .into_inner()
    }

    #[tokio::test]
    async fn catalog_lists_sequenced_contract() {
        let ds = seed_note().await;
        let list = list_contracts(&ds).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].contract_id, "alice");
        assert_eq!(list[0].commits_sequenced, 1);
        assert_eq!(list[0].head.as_deref(), Some("g"));
    }

    #[tokio::test]
    async fn inspect_splits_state_model_rules() {
        let ds = seed_note().await;
        let inspect = inspect_contract(&ds, "alice").await.unwrap().unwrap();
        assert_eq!(inspect.state["/notes/hi.text"], "hello");
        assert!(inspect.model.contains_key("/model/default.modality"));
        assert!(inspect.rules.contains_key("/rules/ok.modality"));
        assert!(inspect.sequenced);
    }

    #[tokio::test]
    async fn unknown_contract_is_none() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        assert!(inspect_contract(&ds, "missing").await.unwrap().is_none());
        assert!(list_commits(&ds, "missing").await.unwrap().is_none());
    }
}
