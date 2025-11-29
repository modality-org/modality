use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use sha2::{Sha256, Digest};

use modal_datastore::DatastoreManager;
use modal_datastore::models::Commit;

use crate::reqres::Response;
use modal_validator_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct PushRequest {
    pub contract_id: String,
    pub commits: Vec<CommitData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitData {
    pub commit_id: String,
    pub body: Value,
    pub head: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PushResponse {
    pub contract_id: String,
    pub pushed_count: usize,
    pub status: String,
}

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: PushRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    let mut saved_count = 0;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    for commit_data in &req.commits {
        let commit_json = serde_json::to_string(&serde_json::json!({
            "body": commit_data.body,
            "head": commit_data.head,
        }))?;
        
        let mut hasher = Sha256::new();
        hasher.update(commit_json.as_bytes());
        let computed_id = format!("{:x}", hasher.finalize());
        
        if computed_id != commit_data.commit_id {
            log::warn!("Commit ID mismatch: expected {}, got {}", commit_data.commit_id, computed_id);
            continue;
        }

        let commit = Commit {
            contract_id: req.contract_id.clone(),
            commit_id: commit_data.commit_id.clone(),
            commit_data: serde_json::to_string(&serde_json::json!({
                "body": commit_data.body,
                "head": commit_data.head,
            }))?,
            timestamp,
            in_batch: None,
        };

        Commit::save_to_final(&commit, datastore_manager).await?;
        saved_count += 1;
    }

    let response = PushResponse {
        contract_id: req.contract_id,
        pushed_count: saved_count,
        status: "pushed".to_string(),
    };

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(response)?),
        errors: None,
    })
}
