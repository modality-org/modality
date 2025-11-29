use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modal_datastore::DatastoreManager;
use modal_datastore::models::Commit;

use crate::reqres::Response;
use modal_validator_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct ListRequest {
    pub contract_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListResponse {
    pub contract_id: String,
    pub commits: Vec<CommitMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitMetadata {
    pub commit_id: String,
    pub timestamp: u64,
    pub in_batch: Option<String>,
}

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: ListRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    let all_commits = Commit::find_by_contract_multi(datastore_manager, &req.contract_id).await?;

    let commits_metadata: Vec<CommitMetadata> = all_commits
        .into_iter()
        .map(|commit| CommitMetadata {
            commit_id: commit.commit_id,
            timestamp: commit.timestamp,
            in_batch: commit.in_batch,
        })
        .collect();

    let response = ListResponse {
        contract_id: req.contract_id,
        commits: commits_metadata,
    };

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(response)?),
        errors: None,
    })
}
