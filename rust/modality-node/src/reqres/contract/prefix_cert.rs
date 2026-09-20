use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use modality_datastore::DatastoreManager;
use modality_validator_consensus::communication::Message as ConsensusMessage;

use crate::reqres::Response;

pub async fn handler(
    data: Option<Value>,
    datastore_manager: &DatastoreManager,
    _consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    let req = data.unwrap_or(json!({}));
    if req
        .get("source_contract")
        .and_then(|v| v.as_str())
        .is_none()
        || req.get("through_commit").and_then(|v| v.as_str()).is_none()
    {
        return Ok(Response {
            ok: false,
            data: None,
            errors: Some(json!({
                "error": "prefix_cert request requires source_contract and through_commit"
            })),
        });
    }
    datastore_manager.enqueue_prefix_cert_request(req)?;
    Ok(Response {
        ok: true,
        data: Some(json!({ "status": "queued" })),
        errors: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queues_request() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let (tx, _rx) = mpsc::channel::<ConsensusMessage>(1);
        let resp = handler(
            Some(json!({
                "source_contract": "src",
                "through_commit": "c1"
            })),
            &mgr,
            tx,
        )
        .await
        .unwrap();
        assert!(resp.ok);
        assert_eq!(mgr.drain_prefix_cert_requests().unwrap().len(), 1);
    }
}
