use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;

use crate::reqres::Response;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct PullRequest {
    pub contract_id: String,
    pub since_commit_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PullResponse {
    pub contract_id: String,
    pub commits: Vec<CommitInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitInfo {
    pub commit_id: String,
    pub body: Value,
    pub head: Value,
    pub timestamp: u64,
}

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: PullRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    let mut all_commits =
        Commit::find_by_contract_multi(datastore_manager, &req.contract_id).await?;
    all_commits.retain(|commit| commit.is_sequenced());
    let all_commits = log_order(all_commits);

    let mut commits_to_return = Vec::new();
    let mut found_since = req.since_commit_id.is_none();

    for commit in all_commits {
        if !found_since {
            if Some(&commit.commit_id) == req.since_commit_id.as_ref() {
                found_since = true;
            }
            continue;
        }

        let commit_data: serde_json::Value =
            serde_json::from_str(&commit.commit_data).unwrap_or_default();
        commits_to_return.push(CommitInfo {
            commit_id: commit.commit_id,
            body: commit_data.get("body").cloned().unwrap_or_default(),
            head: commit_data.get("head").cloned().unwrap_or_default(),
            timestamp: commit.timestamp,
        });
    }

    let response = PullResponse {
        contract_id: req.contract_id,
        commits: commits_to_return,
    };

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(response)?),
        errors: None,
    })
}

/// A contract's commits in log order: each after its parent, the children of
/// one parent by commit id. The store lists them by id, which is no order a
/// puller can apply or resume from.
fn log_order(commits: Vec<Commit>) -> Vec<Commit> {
    let parent_of = |commit: &Commit| -> Option<String> {
        serde_json::from_str::<Value>(&commit.commit_data)
            .ok()?
            .get("head")?
            .get("parent")?
            .as_str()
            .map(str::to_string)
    };
    let ids: std::collections::HashSet<String> =
        commits.iter().map(|c| c.commit_id.clone()).collect();
    let mut children: std::collections::BTreeMap<Option<String>, Vec<Commit>> = Default::default();
    for commit in commits {
        let parent = parent_of(&commit).filter(|p| ids.contains(p));
        children.entry(parent).or_default().push(commit);
    }
    for siblings in children.values_mut() {
        siblings.sort_by(|a, b| b.commit_id.cmp(&a.commit_id));
    }
    let mut ordered = Vec::new();
    let mut stack = children.remove(&None).unwrap_or_default();
    while let Some(commit) = stack.pop() {
        if let Some(mut kids) = children.remove(&Some(commit.commit_id.clone())) {
            stack.append(&mut kids);
        }
        ordered.push(commit);
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn pull_returns_only_sequenced_commits() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        Commit {
            contract_id: "c1".to_string(),
            commit_id: "queued".to_string(),
            commit_data: json!({"body": [], "head": {}}).to_string(),
            timestamp: 1,
            in_batch: None,
        }
        .save_to_final(&mgr)
        .await
        .unwrap();
        Commit {
            contract_id: "c1".to_string(),
            commit_id: "accepted".to_string(),
            commit_data: json!({
                "body": [{ "method": "post", "path": "/notes/ok.text", "value": "yes" }],
                "head": {}
            })
            .to_string(),
            timestamp: 2,
            in_batch: Some("batch-ok".to_string()),
        }
        .save_to_final(&mgr)
        .await
        .unwrap();

        let response = handler(Some(json!({ "contract_id": "c1" })), &mgr, _tx)
            .await
            .unwrap();
        assert!(response.ok);
        let commits = response.data.unwrap()["commits"]
            .as_array()
            .cloned()
            .unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0]["commit_id"], "accepted");
    }

    #[tokio::test]
    async fn pull_returns_the_log_in_order_and_resumes_after_since() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(100);
        // Ids that sort against the log: c comes first, then b, then a.
        for (id, parent) in [("c", None), ("b", Some("c")), ("a", Some("b"))] {
            Commit {
                contract_id: "k".to_string(),
                commit_id: id.to_string(),
                commit_data: json!({"body": [], "head": {"parent": parent}}).to_string(),
                timestamp: 1,
                in_batch: Some("batch".to_string()),
            }
            .save_to_final(&mgr)
            .await
            .unwrap();
        }
        let mut pulled = Vec::new();
        for since in [None, Some("c")] {
            let request = json!({"contract_id": "k", "since_commit_id": since});
            let response = handler(Some(request), &mgr, tx.clone()).await.unwrap();
            pulled.push(
                response.data.unwrap()["commits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c["commit_id"].as_str().unwrap().to_string())
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(pulled[0], ["c", "b", "a"]);
        assert_eq!(pulled[1], ["b", "a"]);
    }
}
