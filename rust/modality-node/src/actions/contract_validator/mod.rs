//! Contract-validator worker: attest a contract prefix through a named commit.
//!
//! Independent of Shoal sequencing. Certificates are opaque sequencer events.

use anyhow::Result;
use modality_common::contract_store::parse_repost_json;
use modality_common::keypair::Keypair;
use modality_datastore::DatastoreManager;
use modality_datastore::models::Commit;
use modality_validator::prefix_cert::{
    PREFIX_CERT_TYPE, PrefixCert, build_prefix_from_store, is_prefix_cert_event, sign_cert,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::node::Node;

/// Run a contract-validator node (networking + prefix-cert worker).
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting contract-validator node");
    maybe_start_worker(node);
    super::observer::run(node).await
}

/// Start the worker if this peer is named or `run_contract_validator` is set.
pub fn maybe_start_worker(node: &Node) {
    let peer_id = node.peerid.to_string();
    let force = node.run_contract_validator;
    let datastore = node.datastore_manager.clone();
    let keypair = match Keypair::from_libp2p_keypair(node.node_keypair.clone()) {
        Ok(kp) => kp,
        Err(e) => {
            log::error!("contract-validator: cannot convert keypair: {}", e);
            return;
        }
    };

    tokio::spawn(async move {
        // Wait until network config is loaded (same process as setup).
        let named = {
            let mgr = datastore.lock().await;
            mgr.contract_validators().unwrap_or_default()
        };
        if !named.contains(&peer_id) && !force {
            log::debug!(
                "contract-validator worker not started (peer {} not in contract_validators)",
                peer_id
            );
            return;
        }
        log::info!("🔏 Contract-validator worker running as {}", peer_id);
        loop {
            if let Err(e) = tick(&datastore, &keypair, &peer_id).await {
                log::warn!("contract-validator tick failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
    });
}

async fn tick(
    datastore: &Arc<Mutex<DatastoreManager>>,
    keypair: &Keypair,
    peer_id: &str,
) -> Result<()> {
    let mut requests = {
        let mgr = datastore.lock().await;
        mgr.drain_prefix_cert_requests()?
    };
    {
        let mgr = datastore.lock().await;
        requests.extend(prefix_cert_requests_from_pending(&mgr).await?);
    }
    for req in requests {
        match issue_cert(datastore, keypair, peer_id, &req).await {
            Ok(Some(event)) => {
                let mgr = datastore.lock().await;
                mgr.enqueue_sequencer_event(event).await?;
            }
            Ok(None) => {}
            Err(e) => log::warn!("prefix_cert request failed: {}", e),
        }
    }
    Ok(())
}

async fn prefix_cert_requests_from_pending(
    mgr: &DatastoreManager,
) -> Result<Vec<serde_json::Value>> {
    let mut out = Vec::new();
    for event in mgr.peek_sequencer_events()? {
        if event.get("type").and_then(|v| v.as_str()) != Some("contract_push") {
            continue;
        }
        let Some(commits) = event
            .get("data")
            .and_then(|d| d.get("commits"))
            .and_then(|c| c.as_array())
        else {
            continue;
        };
        for commit in commits {
            let Some(actions) = commit.get("body").and_then(|b| b.as_array()) else {
                continue;
            };
            for action in actions {
                if let Some(req) = dest_action_prefix_request(mgr, action).await? {
                    out.push(req);
                }
            }
        }
    }
    Ok(out)
}

async fn dest_action_prefix_request(
    mgr: &DatastoreManager,
    action: &serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    let method = action
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    match method.as_str() {
        "repost" => {
            let Ok(spec) = parse_repost_json(action) else {
                return Ok(None);
            };
            Ok(Some(serde_json::json!({
                "source_contract": spec.source_contract,
                "through_commit": spec.source_commit,
                "source_path": spec.source_path,
                "value": spec.value,
            })))
        }
        "recv" => {
            let Some(send_commit_id) = action
                .get("value")
                .and_then(|v| v.get("send_commit_id"))
                .and_then(|v| v.as_str())
                .or_else(|| action.get("send_commit_id").and_then(|v| v.as_str()))
            else {
                return Ok(None);
            };
            let Some(send) = Commit::find_by_id_multi(mgr, send_commit_id).await? else {
                return Ok(None);
            };
            Ok(Some(serde_json::json!({
                "source_contract": send.contract_id,
                "through_commit": send_commit_id,
            })))
        }
        _ => Ok(None),
    }
}

pub async fn issue_cert(
    datastore: &Arc<Mutex<DatastoreManager>>,
    keypair: &Keypair,
    peer_id: &str,
    req: &serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    let source_contract = req
        .get("source_contract")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("prefix_cert request missing source_contract"))?;
    let through_commit = req
        .get("through_commit")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("prefix_cert request missing through_commit"))?;

    {
        let mgr = datastore.lock().await;
        let named = mgr.contract_validators()?;
        if !named.is_empty() && !named.contains(&peer_id.to_string()) {
            anyhow::bail!("this node is not a named contract validator");
        }
        let min_stake = mgr.validator_min_stake()?;
        // Bootstrap stake is treated as 0; named membership is eligibility.
        if min_stake > 0 {
            anyhow::bail!(
                "validator_min_stake is {} but token lock is not implemented in this slice",
                min_stake
            );
        }
        if mgr.has_prefix_cert_from(source_contract, through_commit, peer_id)? {
            return Ok(None);
        }
        if mgr
            .peek_sequencer_events()?
            .iter()
            .any(|event| is_prefix_cert_event(event, source_contract, through_commit, peer_id))
        {
            return Ok(None);
        }
    }

    let (_ids, digest, gas_used) = {
        let mgr = datastore.lock().await;
        build_prefix_from_store(&mgr, source_contract, through_commit).await?
    };
    let fees = {
        let mgr = datastore.lock().await;
        mgr.validation_fees()?
    };
    let mut cert = PrefixCert {
        event_type: PREFIX_CERT_TYPE.to_string(),
        source_contract: source_contract.to_string(),
        through_commit: through_commit.to_string(),
        prefix_digest: digest,
        source_path: req
            .get("source_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        value: req.get("value").cloned(),
        validator_peer_id: peer_id.to_string(),
        gas_used,
        fee_quoted: fees.quote(gas_used),
        requester_peer_id: req
            .get("requester_peer_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        signature: None,
    };
    sign_cert(&mut cert, keypair)?;
    Ok(Some(cert.as_event()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_validator::prefix_cert::peer_id_of;

    #[tokio::test]
    async fn issues_signed_cert_for_local_prefix() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let kp = Keypair::generate().unwrap();
        let peer = peer_id_of(&kp);
        mgr.load_network_config(&serde_json::json!({
            "contract_validators": [peer],
            "validator_min_stake": 0,
            "validation_fees": { "nominal": 1, "meter_coefficient": 2 },
        }))
        .await
        .unwrap();

        let commit = Commit {
            contract_id: "src".into(),
            commit_id: "c1".into(),
            commit_data: serde_json::json!({"body": [], "head": {}}).to_string(),
            timestamp: 1,
            in_batch: Some("b".into()),
        };
        Commit::save_to_final(&commit, &mgr).await.unwrap();

        let ds = Arc::new(Mutex::new(mgr));
        let event = issue_cert(
            &ds,
            &kp,
            &peer,
            &serde_json::json!({
                "source_contract": "src",
                "through_commit": "c1"
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(event["type"], PREFIX_CERT_TYPE);
        assert_eq!(event["gas_used"], 1);
        assert_eq!(event["fee_quoted"], 3);
        modality_validator::prefix_cert::cheap_include_prefix_cert(&event, &[peer]).unwrap();
    }

    #[tokio::test]
    async fn rejects_unnamed_peer() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&serde_json::json!({
            "contract_validators": ["named-peer"],
        }))
        .await
        .unwrap();
        let kp = Keypair::generate().unwrap();
        let ds = Arc::new(Mutex::new(mgr));
        let err = issue_cert(
            &ds,
            &kp,
            &peer_id_of(&kp),
            &serde_json::json!({
                "source_contract": "src",
                "through_commit": "c1"
            }),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("not a named contract validator"));
    }

    #[tokio::test]
    async fn rejects_unsigned_request_path_without_source() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let kp = Keypair::generate().unwrap();
        let ds = Arc::new(Mutex::new(mgr));
        let err = issue_cert(&ds, &kp, &peer_id_of(&kp), &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("source_contract"));
    }

    #[tokio::test]
    async fn second_named_peer_can_issue_after_first_cert_stored() {
        let kp1 = Keypair::generate().unwrap();
        let kp2 = Keypair::generate().unwrap();
        let peer1 = peer_id_of(&kp1);
        let peer2 = peer_id_of(&kp2);
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&serde_json::json!({
            "contract_validators": [peer1, peer2],
        }))
        .await
        .unwrap();
        let commit = Commit {
            contract_id: "src".into(),
            commit_id: "c1".into(),
            commit_data: serde_json::json!({"body": [], "head": {}}).to_string(),
            timestamp: 1,
            in_batch: Some("b".into()),
        };
        Commit::save_to_final(&commit, &mgr).await.unwrap();
        let ds = Arc::new(Mutex::new(mgr));
        let first = issue_cert(
            &ds,
            &kp1,
            &peer1,
            &serde_json::json!({
                "source_contract": "src",
                "through_commit": "c1"
            }),
        )
        .await
        .unwrap()
        .unwrap();
        {
            let mgr = ds.lock().await;
            mgr.save_prefix_cert(&first).unwrap();
        }
        let second = issue_cert(
            &ds,
            &kp2,
            &peer2,
            &serde_json::json!({
                "source_contract": "src",
                "through_commit": "c1"
            }),
        )
        .await
        .unwrap();
        assert!(second.is_some(), "second named signer must still certify");
        let skip_first = issue_cert(
            &ds,
            &kp1,
            &peer1,
            &serde_json::json!({
                "source_contract": "src",
                "through_commit": "c1"
            }),
        )
        .await
        .unwrap();
        assert!(skip_first.is_none(), "same peer should not recertify");
    }

    #[tokio::test]
    async fn pending_dest_recv_requests_prefix_through_send() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let send = Commit {
            contract_id: "src".into(),
            commit_id: "send1".into(),
            commit_data: serde_json::json!({"body": [{"method": "send"}], "head": {}}).to_string(),
            timestamp: 1,
            in_batch: Some("b".into()),
        };
        Commit::save_to_final(&send, &mgr).await.unwrap();
        mgr.enqueue_sequencer_event(serde_json::json!({
            "type": "contract_push",
            "data": {
                "commits": [{
                    "body": [{
                        "method": "recv",
                        "value": { "send_commit_id": "send1" }
                    }]
                }]
            }
        }))
        .await
        .unwrap();
        let reqs = prefix_cert_requests_from_pending(&mgr).await.unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0]["source_contract"], "src");
        assert_eq!(reqs[0]["through_commit"], "send1");
    }
}
