use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use modality_common::contract_store::CommitFile;
use modality_datastore::models::{Commit, Contract};
use modality_datastore::DatastoreManager;
use modality_validator::ContractProcessor;

use crate::actions::sequencer::hash_lane;
use crate::reqres::Response;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

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
    /// The body of a commit whose hash was anchored on the hash lane. It is
    /// sequenced only if it hashes to that commitment.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reveal: bool,
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

    if let Some(refusal) = crate::node::mod_contract::refusal(datastore_manager, &req.contract_id) {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": refusal})),
        });
    }

    let files = match req
        .commits
        .iter()
        .map(|c| CommitFile::verified(&c.commit_id, Some(&c.body), Some(&c.head)))
        .collect::<anyhow::Result<Vec<_>>>()
    {
        Ok(files) => files,
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(json!({"error": e.to_string()})),
            });
        }
    };

    if let Err(e) = reject_unanchored_reveals(datastore_manager, &req) {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": e.to_string()})),
        });
    }

    if let Err(e) = reject_unsequenced_reposts(datastore_manager, &req).await {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({"error": e.to_string()})),
        });
    }

    if let Err(e) = reject_unfunded(datastore_manager, &req).await {
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
        let genesis = match files.first() {
            Some(file) => serde_json::to_string(file)?,
            None => "{}".to_string(),
        };
        Contract {
            contract_id: req.contract_id.clone(),
            genesis,
            created_at: timestamp,
        }
        .save_to_final(datastore_manager)
        .await?;
    }

    let mut queued_commits = Vec::new();
    let mut already_sequenced = 0;
    for (commit_data, file) in req.commits.iter().zip(&files) {
        let keys = [
            ("contract_id".to_string(), req.contract_id.clone()),
            ("commit_id".to_string(), commit_data.commit_id.clone()),
        ]
        .into_iter()
        .collect();
        if let Some(existing) = Commit::find_one_multi(datastore_manager, keys).await? {
            if existing.is_sequenced() {
                already_sequenced += 1;
                continue;
            }
        }

        let commit = Commit {
            contract_id: req.contract_id.clone(),
            commit_id: commit_data.commit_id.clone(),
            commit_data: serde_json::to_string(file)?,
            timestamp,
            in_batch: None,
        };

        Commit::save_to_final(&commit, datastore_manager).await?;
        let mut queued = json!({
            "commit_id": commit_data.commit_id,
            "body": file.body,
            "head": file.head,
        });
        if commit_data.reveal {
            queued["reveal"] = json!(true);
        }
        queued_commits.push(queued);
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

    let status = if saved_count == 0 && already_sequenced > 0 {
        "already_sequenced"
    } else {
        "queued"
    };
    let response = PushResponse {
        contract_id: req.contract_id,
        pushed_count: saved_count,
        status: status.to_string(),
    };

    Ok(Response {
        ok: true,
        data: Some(serde_json::to_value(response)?),
        errors: None,
    })
}

fn reject_unanchored_reveals(
    datastore_manager: &DatastoreManager,
    req: &PushRequest,
) -> anyhow::Result<()> {
    for commit_data in req.commits.iter().filter(|c| c.reveal) {
        let entry = json!({"body": commit_data.body, "head": commit_data.head});
        if let Err(refusal) = hash_lane::check_reveal(
            datastore_manager,
            &req.contract_id,
            &commit_data.commit_id,
            &entry,
        ) {
            anyhow::bail!(
                "reveal of {} refused: {}{}",
                commit_data.commit_id,
                refusal,
                if refusal == hash_lane::RevealRefusal::NotAnchored {
                    " on this node yet; anchor it with `modal contract anchor` and wait for it to be certified"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}

/// On a network that prices gas, each commit names a payer that signed it
/// and holds the most the commit can cost (counting MOD the push receives
/// into the payer's own contract). Checked again when the commit is applied.
async fn reject_unfunded(mgr: &DatastoreManager, req: &PushRequest) -> anyhow::Result<()> {
    use crate::actions::sequencer::consensus::{fee_payer, push_receipts};
    if !mgr.gas_price()?.is_priced() {
        return Ok(());
    }
    let entries: Vec<Value> = req
        .commits
        .iter()
        .map(|c| json!({"commit_id": c.commit_id, "body": c.body, "head": c.head}))
        .collect();
    let receipts = push_receipts(mgr, &req.contract_id, &entries).await?;
    // The block that orders these is not made yet; check at the base this
    // node would state next. Apply checks again at the block's own base.
    let base = crate::actions::sequencer::consensus::own_next_base(mgr);
    for entry in &entries {
        fee_payer(mgr, &req.contract_id, entry, receipts, base).await?;
    }
    Ok(())
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

    fn id_of(body: Value, head: Value) -> String {
        serde_json::from_value::<CommitFile>(json!({ "body": body, "head": head }))
            .unwrap()
            .compute_id()
            .unwrap()
    }

    #[tokio::test]
    async fn test_push_accepts_cli_hash_data_fields_and_queues() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        let body = json!([{"method": "post", "path": "/hello.txt", "value": "hi"}]);
        let data = json!({
            "contract_id": "test-contract",
            "commits": [{
                "hash": id_of(body.clone(), json!({})),
                "data": body,
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
    async fn test_repush_keeps_sequenced_commit_and_does_not_requeue_it() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(100);
        let genesis_body = json!([{"method": "post", "path": "/a.text", "value": "a"}]);
        let genesis_id = id_of(genesis_body.clone(), json!({}));
        let next_body = json!([{"method": "post", "path": "/b.text", "value": "b"}]);
        let next_id = id_of(next_body.clone(), json!({"parent": genesis_id}));
        let push = json!({
            "contract_id": "c",
            "commits": [
                {"commit_id": genesis_id, "body": genesis_body, "head": {}},
                {"commit_id": next_id, "body": next_body, "head": {"parent": genesis_id}}
            ]
        });

        assert!(
            handler(Some(push.clone()), &mgr, tx.clone())
                .await
                .unwrap()
                .ok
        );
        mgr.drain_sequencer_events().await.unwrap();
        let keys = [
            ("contract_id".to_string(), "c".to_string()),
            ("commit_id".to_string(), genesis_id.clone()),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let mut genesis = Commit::find_one_multi(&mgr, keys.clone())
            .await
            .unwrap()
            .unwrap();
        genesis.in_batch = Some("batch-1".into());
        genesis.save_to_final(&mgr).await.unwrap();

        let response = handler(Some(push), &mgr, tx).await.unwrap();
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["pushed_count"], 1);

        let genesis = Commit::find_one_multi(&mgr, keys).await.unwrap().unwrap();
        assert_eq!(genesis.in_batch.as_deref(), Some("batch-1"));

        let events = mgr.drain_sequencer_events().await.unwrap();
        assert_eq!(events.len(), 1);
        let commits = events[0]["data"]["commits"].as_array().unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0]["commit_id"], json!(next_id));
    }

    #[tokio::test]
    async fn test_push_refuses_a_commit_that_does_not_hash_to_its_id() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(100);
        let body = json!([{"method": "post", "path": "/a.text", "value": "a"}]);
        let genuine = id_of(body.clone(), json!({}));
        let other = json!([{"method": "post", "path": "/a.text", "value": "forged"}]);

        for (commit_id, body) in [(genuine.as_str(), other), ("made-up", body)] {
            let data = json!({
                "contract_id": "c",
                "commits": [{"commit_id": commit_id, "body": body, "head": {}}]
            });
            let response = handler(Some(data), &mgr, tx.clone()).await.unwrap();
            assert!(!response.ok);
            let err = response.errors.unwrap().to_string();
            assert!(
                err.contains("is not what its body and head hash to"),
                "{err}"
            );
        }
        assert!(mgr.drain_sequencer_events().await.unwrap().is_empty());
        assert!(Contract::find_by_id_multi(&mgr, "c")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_push_rejects_repost_of_unsequenced_source() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (_tx, _rx) = mpsc::channel::<ConsensusMessage>(100);

        let body = json!([{
            "method": "repost",
            "path": "/reposts/src/hello.text",
            "value": "secret",
            "source_contract": "src",
            "source_path": "/hello.text",
            "source_commit": "src-commit"
        }]);
        let data = json!({
            "contract_id": "dest",
            "commits": [{
                "commit_id": id_of(body.clone(), json!({})),
                "body": body,
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
