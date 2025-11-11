use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modal_datastore::NetworkDatastore;
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
    datastore: &NetworkDatastore,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req: ListRequest = if let Some(d) = data {
        serde_json::from_value(d)?
    } else {
        anyhow::bail!("Missing request data");
    };

    // Get all commits for this contract
    let all_commits = Commit::find_by_contract(datastore, &req.contract_id).await?;

    // Extract metadata
    let commits_metadata: Vec<CommitMetadata> = all_commits
        .into_iter()
        .map(|commit| CommitMetadata {
            commit_id: commit.commit_id,
            timestamp: commit.timestamp,
            in_batch: commit.in_batch,
        })
        .collect();

    let response = ListResponse {
        contract_id: req.contract_id.clone(),
        commits: commits_metadata,
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
    async fn test_list_commits() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel(100);

        // Save test commits
        for i in 0..3 {
            let commit = Commit {
                contract_id: "test_contract_123".to_string(),
                commit_id: format!("commit_{}", i),
                commit_data: serde_json::to_string(&serde_json::json!({
                    "body": [],
                    "head": {}
                })).unwrap(),
                timestamp: 1234567890 + i,
                in_batch: None,
            };
            commit.save(&datastore).await.unwrap();
        }

        let data = serde_json::json!({
            "contract_id": "test_contract_123"
        });

        let result = handler(Some(data), &datastore, tx).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.ok);
        
        let response_data: ListResponse = serde_json::from_value(
            response.data.unwrap()
        ).unwrap();
        assert_eq!(response_data.commits.len(), 3);
    }
}

