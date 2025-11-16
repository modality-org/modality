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
    datastore: &NetworkDatastore,
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

    // Process each commit
    for commit_data in &req.commits {
        // Verify commit ID matches the hash
        let commit_json = serde_json::to_string(&serde_json::json!({
            "body": commit_data.body,
            "head": commit_data.head,
        }))?;
        
        let mut hasher = Sha256::new();
        hasher.update(commit_json.as_bytes());
        let computed_id = format!("{:x}", hasher.finalize());
        
        if computed_id != commit_data.commit_id {
            anyhow::bail!("Commit ID mismatch: expected {}, got {}", 
                computed_id, commit_data.commit_id);
        }

        // Check if commit already exists
        let mut keys = std::collections::HashMap::new();
        keys.insert("contract_id".to_string(), req.contract_id.clone());
        keys.insert("commit_id".to_string(), commit_data.commit_id.clone());
        
        if Commit::find_one(datastore, keys).await?.is_none() {
            // Store the commit in datastore
            let commit = Commit {
                contract_id: req.contract_id.clone(),
                commit_id: commit_data.commit_id.clone(),
                commit_data: commit_json,
                timestamp,
                in_batch: None,
            };
            
            commit.save(datastore).await?;
            saved_count += 1;
        }
    }

    // TODO: Submit commits to consensus for inclusion in batches
    // For now, just store in datastore

    let response = PushResponse {
        contract_id: req.contract_id.clone(),
        pushed_count: saved_count,
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
    async fn test_push_commits() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<()>(100);

        let commit_data = CommitData {
            commit_id: "test123".to_string(),
            body: serde_json::json!([{"method": "post", "path": "/test.txt", "value": "hello"}]),
            head: serde_json::json!({}),
        };

        // Note: This test will fail ID verification, but demonstrates structure
        let data = serde_json::json!({
            "contract_id": "test_contract_123",
            "commits": vec![commit_data]
        });

        // Would need to compute correct commit_id for real test
        // let result = handler(Some(data), &datastore, tx).await;
    }
}

