use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use sha2::{Sha256, Digest};

use modal_datastore::NetworkDatastore;
use modal_datastore::models::Commit;
use modal_datastore::model::Model;

use crate::reqres::Response;
use modal_validator_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitCommitRequest {
    pub contract_id: String,
    pub commit_data: Value, // {body: [...], head: {...}}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitCommitResponse {
    pub commit_id: String,
    pub contract_id: String,
    pub status: String,
}

pub async fn handler(
    data: Option<Value>,
    datastore: &NetworkDatastore,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: SubmitCommitRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    // Generate commit ID from the commit data hash
    let commit_json = serde_json::to_string(&req.commit_data)?;
    let mut hasher = Sha256::new();
    hasher.update(commit_json.as_bytes());
    let commit_id = format!("{:x}", hasher.finalize());
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Store the commit in datastore
    let commit = Commit {
        contract_id: req.contract_id.clone(),
        commit_id: commit_id.clone(),
        commit_data: commit_json.clone(),
        timestamp,
        in_batch: None,
    };
    
    commit.save(datastore).await?;

    // TODO: Submit to consensus when ShoalValidator is integrated
    // For now, just store in datastore
    // let transaction = Transaction {
    //     data: commit_json.into_bytes(),
    //     timestamp,
    // };
    // validator.submit_transaction(transaction).await?;

    let response = SubmitCommitResponse {
        commit_id: commit_id.clone(),
        contract_id: req.contract_id.clone(),
        status: "stored".to_string(),
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

    #[tokio::test]
    async fn test_submit_commit() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel(100);

        let commit_data = serde_json::json!({
            "body": [{"method": "post", "path": "/test.txt", "value": "hello world"}],
            "head": {}
        });

        let data = serde_json::json!({
            "contract_id": "test_contract_123",
            "commit_data": commit_data
        });

        let result = handler(Some(data), &datastore, tx).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.ok);
    }
}

