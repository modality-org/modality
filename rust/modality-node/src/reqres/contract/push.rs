use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use modality_datastore::models::{Commit, Contract};
use modality_datastore::DatastoreManager;
use modality_validator::ContractProcessor;

use crate::reqres::Response;
use modality_validator_consensus::communication::Message as ConsensusMessage;

#[derive(Serialize, Deserialize, Debug)]
pub struct PushRequest {
    pub contract_id: String,
    pub commits: Vec<CommitData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitData {
    #[serde(alias = "hash")]
    pub commit_id: String,
    #[serde(alias = "data")]
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

    if let Err(e) = reject_unsequenced_reposts(datastore_manager, &req).await {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": e.to_string()})),
        });
    }

    let mut saved_count = 0;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    if Contract::find_by_id_multi(datastore_manager, &req.contract_id)
        .await?
        .is_none()
    {
        let genesis = req
            .commits
            .first()
            .map(|c| {
                json!({
                    "body": c.body,
                    "head": c.head,
                })
                .to_string()
            })
            .unwrap_or_else(|| "{}".to_string());
        Contract {
            contract_id: req.contract_id.clone(),
            genesis,
            created_at: timestamp,
        }
        .save_to_final(datastore_manager)
        .await?;
    }

    let mut queued_commits = Vec::new();
    for commit_data in &req.commits {
        let commit_data_json = json!({
            "body": commit_data.body,
            "head": commit_data.head,
        });

        let commit = Commit {
            contract_id: req.contract_id.clone(),
            commit_id: commit_data.commit_id.clone(),
            commit_data: commit_data_json.to_string(),
            timestamp,
            in_batch: None,
        };

        Commit::save_to_final(&commit, datastore_manager).await?;
        queued_commits.push(json!({
            "commit_id": commit_data.commit_id,
            "body": commit_data.body,
            "head": commit_data.head,
        }));
        saved_count += 1;
    }

    if !queued_commits.is_empty() {
        datastore_manager
            .enqueue_sequencer_event(json!({
                "type": "contract_push",
                "data": {
                    "contract_id": req.contract_id,
                    "commits": queued_commits,
                }
            }))
            .await?;
    }

    let response = PushResponse {
        contract_id: req.contract_id,
        pushed_count: saved_count,
        status: "queued".to_string(),
    };

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(response)?),
        errors: None,
    })
}

async fn reject_unsequenced_reposts(
    datastore_manager: &DatastoreManager,
    req: &PushRequest,
) -> anyhow::Result<()> {
    for commit_data in &req.commits {
        let Some(actions) = commit_data.body.as_array() else {
            continue;
        };
        for action in actions {
            let method = action
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            if method != "repost" {
                continue;
            }
            let spec = modality_common::contract_store::parse_repost_json(action)?;
            ContractProcessor::assert_repost_source_sequenced(datastore_manager, &spec).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_push_accepts_cli_hash_data_fields_and_queues() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        let data = json!({
            "contract_id": "test-contract",
            "commits": [{
                "hash": "abc123",
                "data": [{"method": "post", "path": "/hello.txt", "value": "hi"}],
                "head": {"parent": null}
            }]
        });

        let response = handler(Some(data), &mgr, _tx).await.unwrap();
        assert!(response.ok);
        let body = response.data.unwrap();
        assert_eq!(body["pushed_count"], 1);
        assert_eq!(body["status"], "queued");

        let events = mgr.drain_sequencer_events().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "contract_push");
    }

    #[tokio::test]
    async fn test_push_rejects_repost_of_unsequenced_source() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        let data = json!({
            "contract_id": "dest",
            "commits": [{
                "commit_id": "dest-commit",
                "body": [{
                    "method": "repost",
                    "path": "/reposts/src/hello.text",
                    "value": "secret",
                    "source_contract": "src",
                    "source_path": "/hello.text",
                    "source_commit": "src-commit"
                }],
                "head": {}
            }]
        });

        let response = handler(Some(data), &mgr, _tx).await.unwrap();
        assert!(!response.ok);
        let err = response.errors.unwrap().to_string();
        assert!(
            err.contains("was not found") || err.contains("has not been sequenced"),
            "unexpected error: {err}"
        );
        assert!(mgr.drain_sequencer_events().await.unwrap().is_empty());
    }
}
