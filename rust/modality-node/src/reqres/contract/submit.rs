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
pub struct SubmitCommitRequest {
    pub contract_id: String,
    pub commit_data: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitCommitResponse {
    pub commit_id: String,
    pub contract_id: String,
    pub status: String,
}

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: SubmitCommitRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    let commit_json = serde_json::to_string(&req.commit_data)?;
    let mut hasher = Sha256::new();
    hasher.update(commit_json.as_bytes());
    let commit_id = format!("{:x}", hasher.finalize());
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let commit = Commit {
        contract_id: req.contract_id.clone(),
        commit_id: commit_id.clone(),
        commit_data: serde_json::to_string(&req.commit_data)?,
        timestamp,
        in_batch: None,
    };

    // Save to ValidatorFinal store
    Commit::save_to_final(&commit, datastore_manager).await?;

    let response = SubmitCommitResponse {
        commit_id,
        contract_id: req.contract_id,
        status: "submitted".to_string(),
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

    #[tokio::test]
    async fn test_submit_commit() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);
        
        let data = serde_json::json!({
            "contract_id": "test-contract",
            "commit_data": {
                "body": ["add", "x", 1],
                "head": {"version": 1}
            }
        });
        
        let response = handler(Some(data), &mgr, _tx).await.unwrap();
        assert!(response.ok);
    }
}
