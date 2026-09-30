//! Hash-lane requests: the network's parameters and anchors, submitting a
//! record, and reading what was anchored.

use anyhow::Result;
use serde_json::{json, Value};

use modality_common::hash_commitment::HashCommitment;
use modality_datastore::models::Commit;
use modality_datastore::DatastoreManager;

use crate::actions::sequencer::hash_lane::anchor_window;
use crate::reqres::Response;

pub const PARAMS_PATH: &str = "/hash_commitment/params";
pub const SUBMIT_PATH: &str = "/hash_commitment/submit";
pub const GET_PATH: &str = "/hash_commitment/get";

fn refused(error: impl std::fmt::Display) -> Response {
    Response {
        ok: false,
        data: None,
        errors: Some(json!({"error": error.to_string()})),
    }
}

fn answered(data: Value) -> Response {
    Response {
        ok: true,
        data: Some(data),
        errors: None,
    }
}

/// Parameters and the anchor a new record should bind to.
pub async fn params_handler(mgr: &DatastoreManager) -> Result<Response> {
    let Some(params) = mgr.hash_lane_params()? else {
        return Ok(answered(json!({"enabled": false})));
    };
    let window = anchor_window(mgr).await?;
    Ok(answered(json!({
        "enabled": true,
        "params": params,
        "current": window.current(),
        "window": window,
    })))
}

/// Queue a record for this node's next sequencer block. It is checked here as
/// the committee will check it, so a record that would cost the block its
/// votes is refused now.
pub async fn submit_handler(data: Option<Value>, mgr: &DatastoreManager) -> Result<Response> {
    let Some(params) = mgr.hash_lane_params()? else {
        return Ok(refused("this network has no hash lane"));
    };
    let record: HashCommitment = match data.map(serde_json::from_value).transpose() {
        Ok(Some(record)) => record,
        Ok(None) => return Ok(refused("missing hash commitment")),
        Err(e) => return Ok(refused(format!("not a hash commitment: {e}"))),
    };
    let window = anchor_window(mgr).await?;
    let signer_set = mgr.hash_signer_set(&record.contract_id)?;
    if let Err(e) = record.verify_for_vote(&params, &window, signer_set.as_deref()) {
        return Ok(refused(e));
    }
    if let Some(existing) = mgr.hash_commitment(&record.contract_id, &record.commit_id)? {
        return Ok(answered(json!({"status": "anchored", "record": existing})));
    }
    mgr.enqueue_sequencer_event(record.to_event()?).await?;
    Ok(answered(json!({
        "status": "queued",
        "contract_id": record.contract_id,
        "commit_id": record.commit_id,
        "work_bits": record.work_bits(),
    })))
}

/// What this node indexed for a contract, or for one commit of it, and
/// whether the commit's body has been sequenced.
pub async fn get_handler(data: Option<Value>, mgr: &DatastoreManager) -> Result<Response> {
    let data = data.unwrap_or(Value::Null);
    let Some(contract_id) = data.get("contract_id").and_then(Value::as_str) else {
        return Ok(refused("hash_commitment/get requires contract_id"));
    };
    let Some(commit_id) = data.get("commit_id").and_then(Value::as_str) else {
        let records = mgr.hash_commitments_for(contract_id)?;
        return Ok(answered(json!({"contract_id": contract_id, "records": records})));
    };
    let record = mgr.hash_commitment(contract_id, commit_id)?;
    let keys = [
        ("contract_id".to_string(), contract_id.to_string()),
        ("commit_id".to_string(), commit_id.to_string()),
    ]
    .into_iter()
    .collect();
    let sequenced = Commit::find_one_multi(mgr, keys)
        .await?
        .map(|c| c.is_sequenced())
        .unwrap_or(false);
    let status = match (&record, sequenced) {
        (Some(_), true) => "revealed",
        (Some(_), false) => "anchored",
        (None, true) => "sequenced",
        (None, false) => "unknown",
    };
    Ok(answered(json!({
        "contract_id": contract_id,
        "commit_id": commit_id,
        "status": status,
        "record": record,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_common::keypair::Keypair;

    async fn lane_mgr() -> DatastoreManager {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&json!({
            "name": "t",
            "hash_lane": {"quota_per_block": 4, "floor_bits": 6}
        }))
        .await
        .unwrap();
        mgr
    }

    #[tokio::test]
    async fn a_ground_record_is_queued_and_a_weak_one_refused() {
        let mgr = lane_mgr().await;
        let params = params_handler(&mgr).await.unwrap().data.unwrap();
        assert_eq!(params["enabled"], true);
        let anchor = serde_json::from_value(params["current"].clone()).unwrap();

        let keypair = Keypair::generate().unwrap();
        let mut record = HashCommitment::signed(&keypair, "c", &"11".repeat(32), None).unwrap();
        record.grind(&anchor, 6).unwrap();
        let ok = submit_handler(Some(serde_json::to_value(&record).unwrap()), &mgr)
            .await
            .unwrap();
        assert!(ok.ok, "{:?}", ok.errors);
        let events = mgr.drain_sequencer_events().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "hash_commitment");

        let mut weak = record.clone();
        while weak.work_bits() >= 6 {
            weak.nonce += 1;
        }
        let refused = submit_handler(Some(serde_json::to_value(&weak).unwrap()), &mgr)
            .await
            .unwrap();
        assert!(!refused.ok);
        assert!(mgr.drain_sequencer_events().await.unwrap().is_empty());

        let got = get_handler(
            Some(json!({"contract_id": "c", "commit_id": record.commit_id})),
            &mgr,
        )
        .await
        .unwrap()
        .data
        .unwrap();
        assert_eq!(got["status"], "unknown");
    }

    #[tokio::test]
    async fn a_network_without_the_lane_refuses_records() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        assert_eq!(params_handler(&mgr).await.unwrap().data.unwrap()["enabled"], false);
        let keypair = Keypair::generate().unwrap();
        let record = HashCommitment::signed(&keypair, "c", &"11".repeat(32), None).unwrap();
        let response = submit_handler(Some(serde_json::to_value(&record).unwrap()), &mgr)
            .await
            .unwrap();
        assert!(!response.ok);
    }
}
