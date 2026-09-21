use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;

use crate::reqres::Response;
use modality_validator_consensus::communication::Message as ConsensusMessage;

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
}
