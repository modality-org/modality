//! Read the prefix certs this node holds for one source commit, so a node
//! catching up can apply the dest `RECV` / `REPOST` that waited on them.
//! Certs are signed; the caller checks each before storing it.

use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

use crate::reqres::Response;

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req = data.unwrap_or(json!({}));
    let Some(through_commit) = req.get("through_commit").and_then(|v| v.as_str()) else {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({ "error": "prefix_certs request requires through_commit" })),
        });
    };
    let source_contract = match req.get("source_contract").and_then(|v| v.as_str()) {
        Some(contract) => Some(contract.to_string()),
        None => Commit::find_by_id_multi(datastore_manager, through_commit)
            .await?
            .map(|commit| commit.contract_id),
    };
    let certs = match &source_contract {
        Some(contract) => datastore_manager.list_prefix_certs(contract, through_commit)?,
        None => Vec::new(),
    };
    Ok(Response {
        ok: true,
        data: Some(json!({
            "source_contract": source_contract,
            "through_commit": through_commit,
            "certs": certs,
        })),
        errors: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lists_certs_by_source_commit() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(1);
        Commit {
            contract_id: "src".into(),
            commit_id: "c1".into(),
            commit_data: "{}".into(),
            timestamp: 1,
            in_batch: Some("b".into()),
        }
        .save_to_final(&mgr)
        .await
        .unwrap();
        mgr.save_prefix_cert(&json!({
            "type": "prefix_cert",
            "source_contract": "src",
            "through_commit": "c1",
            "prefix_digest": "d",
            "validator_peer_id": "v1",
            "gas_used": 1,
            "fee_quoted": 0
        }))
        .unwrap();
        let resp = handler(Some(json!({ "through_commit": "c1" })), &mgr, tx.clone())
            .await
            .unwrap();
        let data = resp.data.unwrap();
        assert_eq!(data["source_contract"], "src");
        assert_eq!(data["certs"].as_array().unwrap().len(), 1);

        let unknown = handler(Some(json!({ "through_commit": "nope" })), &mgr, tx)
            .await
            .unwrap();
        assert!(unknown.data.unwrap()["certs"].as_array().unwrap().is_empty());
    }
}
