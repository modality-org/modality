use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modal_datastore::NetworkDatastore;
use modal_datastore::models::Commit;

use crate::reqres::Response;
use modal_validator_consensus::communication::Message as ConsensusMessage;

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
    datastore: &NetworkDatastore,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: PullRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    // Get all commits for this contract
    let all_commits = Commit::find_by_contract(datastore, &req.contract_id).await?;

    // Filter commits if since_commit_id is provided
    let mut commits_to_return = Vec::new();
    let mut found_since = req.since_commit_id.is_none(); // If no since_commit, include all

    for commit in all_commits {
        if !found_since {
            if Some(&commit.commit_id) == req.since_commit_id.as_ref() {
                found_since = true;
            }
            continue; // Skip commits before since_commit_id
        }

        // Parse commit data
        let commit_data: Value = serde_json::from_str(&commit.commit_data)?;
        let body = commit_data.get("body")
            .cloned()
            .unwrap_or(Value::Null);
        let head = commit_data.get("head")
            .cloned()
            .unwrap_or(Value::Null);

        commits_to_return.push(CommitInfo {
            commit_id: commit.commit_id,
            body,
            head,
            timestamp: commit.timestamp,
        });
    }

    let response = PullResponse {
        contract_id: req.contract_id.clone(),
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
    use modal_datastore::NetworkDatastore;
    use modal_datastore::models::Commit;
    use modal_datastore::model::Model;

    #[tokio::test]
    async fn test_pull_commits() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel(100);

        // Save a test commit
        let commit = Commit {
            contract_id: "test_contract_123".to_string(),
            commit_id: "commit_abc".to_string(),
            commit_data: serde_json::to_string(&serde_json::json!({
                "body": [{"method": "post", "path": "/test", "value": "data"}],
                "head": {}
            })).unwrap(),
            timestamp: 1234567890,
            in_batch: None,
        };
        commit.save(&datastore).await.unwrap();

        let data = serde_json::json!({
            "contract_id": "test_contract_123",
            "since_commit_id": null
        });

        let result = handler(Some(data), &datastore, tx).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.ok);
        
        let response_data: PullResponse = serde_json::from_value(
            response.data.unwrap()
        ).unwrap();
        assert_eq!(response_data.commits.len(), 1);
        assert_eq!(response_data.commits[0].commit_id, "commit_abc");
    }
}

