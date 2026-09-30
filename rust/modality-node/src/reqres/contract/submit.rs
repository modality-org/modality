use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use modality_common::contract_store::CommitFile;
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;

use crate::reqres::Response;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

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

    // Stored under the id its body and head hash to, as a push is.
    let file: CommitFile = serde_json::from_value(req.commit_data)
        .map_err(|e| anyhow::anyhow!("commit_data is not a commit: {e}"))?;
    let commit_id = file.compute_id()?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let commit = Commit {
        contract_id: req.contract_id.clone(),
        commit_id: commit_id.clone(),
        commit_data: serde_json::to_string(&file)?,
        timestamp,
        in_batch: None,
    };

    // Save to SequencerFinal store
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
                "body": [{"method": "post", "path": "/a.text", "value": "a"}],
                "head": {}
            }
        });

        let response = handler(Some(data), &mgr, _tx.clone()).await.unwrap();
        assert!(response.ok);
        let mut file = CommitFile::new();
        file.add_action("post".into(), Some("/a.text".into()), serde_json::json!("a"));
        assert_eq!(
            response.data.unwrap()["commit_id"],
            serde_json::json!(file.compute_id().unwrap())
        );

        let not_a_commit = serde_json::json!({
            "contract_id": "test-contract",
            "commit_data": {"body": ["add", "x", 1], "head": {}}
        });
        assert!(handler(Some(not_a_commit), &mgr, _tx).await.is_err());
    }
}
