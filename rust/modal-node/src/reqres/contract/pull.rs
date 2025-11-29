use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modal_datastore::DatastoreManager;
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
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: PullRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    let all_commits = Commit::find_by_contract_multi(datastore_manager, &req.contract_id).await?;

    let mut commits_to_return = Vec::new();
    let mut found_since = req.since_commit_id.is_none();

    for commit in all_commits {
        if !found_since {
            if Some(&commit.commit_id) == req.since_commit_id.as_ref() {
                found_since = true;
            }
            continue;
        }

        let commit_data: serde_json::Value = serde_json::from_str(&commit.commit_data).unwrap_or_default();
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
