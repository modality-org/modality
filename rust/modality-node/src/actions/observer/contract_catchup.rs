//! Pull sequenced contract logs onto an observer (or any non-sequencer).
//!
//! Live path: certified sequencer-block gossip, then the same apply as sequencers.
//! Historical path: `/contract/catalog` + list/pull from a bootstrapper.

use anyhow::Result;
use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use modality_datastore::models::{Commit, Contract};
use modality_datastore::DatastoreManager;
use modality_validator::ContractProcessor;
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

use crate::actions::sequencer::consensus::apply_certified_contract_events;
use crate::constants::REQRES_TIMEOUT_SECS;
use crate::node::Node;
use crate::reqres;
use crate::sync::common_ancestor::wait_for_reqres_response;

const CATCHUP_INTERVAL_SECS: u64 = 30;

pub fn start_live_apply(node: &mut Node) {
    let Some(mut rx) = node.take_consensus_rx() else {
        log::warn!("Observer contract catch-up: consensus receiver already taken");
        return;
    };
    let datastore = node.datastore_manager.clone();
    tokio::spawn(async move {
        log::info!("Observer applying certified contract_push events");
        while let Some(msg) = rx.recv().await {
            if let ConsensusMessage::CertifiedSequencerBlock { block, .. } = msg {
                if block.cert.is_some() {
                    apply_certified_contract_events(&block, &datastore).await;
                }
            }
        }
    });
}

pub fn start_historical_pull(node: &Node) {
    let swarm = node.swarm.clone();
    let datastore = node.datastore_manager.clone();
    let reqres_txs = node.reqres_response_txs.clone();
    let bootstrappers = node.bootstrappers.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CATCHUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            if bootstrappers.is_empty() {
                continue;
            }
            for ma in &bootstrappers {
                match pull_from_peer(&swarm, &reqres_txs, ma, &datastore).await {
                    Ok(n) if n > 0 => {
                        log::info!("Contract catch-up applied {n} sequenced commit(s) from {ma}");
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        log::debug!("Contract catch-up from {ma}: {e}");
                    }
                }
            }
        }
    });
}

async fn pull_from_peer(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    reqres_txs: &Arc<
        Mutex<
            HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
    peer_addr: &Multiaddr,
    datastore: &Arc<Mutex<DatastoreManager>>,
) -> Result<usize> {
    let catalog = request_json(swarm, reqres_txs, peer_addr, "/contract/catalog", None).await?;
    let contracts = catalog
        .as_array()
        .cloned()
        .or_else(|| catalog.get("contracts").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();
    let mut applied = 0usize;
    for entry in contracts {
        let Some(contract_id) = entry
            .get("contract_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| entry.as_str().map(str::to_string))
        else {
            continue;
        };
        applied += catchup_contract(swarm, reqres_txs, peer_addr, datastore, &contract_id).await?;
    }
    Ok(applied)
}

async fn catchup_contract(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    reqres_txs: &Arc<
        Mutex<
            HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
    peer_addr: &Multiaddr,
    datastore: &Arc<Mutex<DatastoreManager>>,
    contract_id: &str,
) -> Result<usize> {
    let list = request_json(
        swarm,
        reqres_txs,
        peer_addr,
        "/contract/list",
        Some(serde_json::json!({ "contract_id": contract_id })),
    )
    .await?;
    let pull = request_json(
        swarm,
        reqres_txs,
        peer_addr,
        "/contract/pull",
        Some(serde_json::json!({ "contract_id": contract_id })),
    )
    .await?;

    let mut in_batch: HashMap<String, String> = HashMap::new();
    if let Some(commits) = list.get("commits").and_then(|v| v.as_array()) {
        for commit in commits {
            if let (Some(id), Some(batch)) = (
                commit.get("commit_id").and_then(|v| v.as_str()),
                commit.get("in_batch").and_then(|v| v.as_str()),
            ) {
                if !batch.is_empty() {
                    in_batch.insert(id.to_string(), batch.to_string());
                }
            }
        }
    }

    let Some(commits) = pull.get("commits").and_then(|v| v.as_array()) else {
        return Ok(0);
    };

    {
        let mgr = datastore.lock().await;
        if Contract::find_by_id_multi(&mgr, contract_id)
            .await?
            .is_none()
        {
            let genesis = commits
                .first()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "{}".to_string());
            let created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Contract {
                contract_id: contract_id.to_string(),
                genesis,
                created_at,
            }
            .save_to_final(&mgr)
            .await?;
        }
    }

    let processor = ContractProcessor::new(datastore.clone());
    let mut applied = 0usize;
    let mut remaining: Vec<Value> = commits.clone();
    let mut fetched_certs_for: std::collections::HashSet<String> = Default::default();
    let mut progressed = true;
    while progressed && !remaining.is_empty() {
        progressed = false;
        let mut next = Vec::new();
        for commit_entry in remaining {
            let Some(commit_id) = commit_entry
                .get("commit_id")
                .or_else(|| commit_entry.get("hash"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            {
                let mgr = datastore.lock().await;
                let keys = [
                    ("contract_id".to_string(), contract_id.to_string()),
                    ("commit_id".to_string(), commit_id.clone()),
                ]
                .into_iter()
                .collect();
                if let Ok(Some(existing)) = Commit::find_one_multi(&mgr, keys).await {
                    if existing.is_sequenced() {
                        continue;
                    }
                }
            }
            let commit_data = serde_json::json!({
                "body": commit_entry.get("body"),
                "head": commit_entry.get("head"),
            });
            match processor
                .process_commit(contract_id, &commit_id, &commit_data.to_string())
                .await
            {
                Ok(_) => {
                    if let Some(batch) = in_batch.get(&commit_id) {
                        let mgr = datastore.lock().await;
                        let keys = [
                            ("contract_id".to_string(), contract_id.to_string()),
                            ("commit_id".to_string(), commit_id.clone()),
                        ]
                        .into_iter()
                        .collect();
                        if let Ok(Some(mut commit)) = Commit::find_one_multi(&mgr, keys).await {
                            commit.in_batch = Some(batch.clone());
                            let _ = commit.save_to_final(&mgr).await;
                        }
                    }
                    applied += 1;
                    progressed = true;
                }
                Err(err) => {
                    if err.to_string().contains("missing prefix_cert")
                        && fetched_certs_for.insert(commit_id.clone())
                    {
                        let stored =
                            fetch_prefix_certs(swarm, reqres_txs, peer_addr, datastore, &commit_entry)
                                .await
                                .unwrap_or_else(|e| {
                                    log::debug!("Prefix certs for {commit_id}: {e}");
                                    0
                                });
                        if stored > 0 {
                            progressed = true;
                        }
                    }
                    next.push(commit_entry)
                }
            }
        }
        remaining = next;
    }
    Ok(applied)
}

/// Source commits a dest `RECV` / `REPOST` in this commit waits on, as
/// `(source contract if the action names it, through commit)`.
fn prefix_cert_sources(commit_entry: &Value) -> Vec<(Option<String>, String)> {
    let Some(actions) = commit_entry.get("body").and_then(|b| b.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for action in actions {
        let method = action
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        match method.as_str() {
            "recv" => {
                if let Some(id) = action
                    .get("value")
                    .and_then(|v| v.get("send_commit_id"))
                    .and_then(|v| v.as_str())
                {
                    out.push((None, id.to_string()));
                }
            }
            "repost" => {
                if let Ok(spec) = modality_common::contract_store::parse_repost_json(action) {
                    out.push((Some(spec.source_contract), spec.source_commit));
                }
            }
            _ => {}
        }
    }
    out
}

/// Fetch the prefix certs a dest commit waits on from the peer, and store
/// those a named validator signed. Returns how many were new.
async fn fetch_prefix_certs(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    reqres_txs: &Arc<
        Mutex<
            HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
    peer_addr: &Multiaddr,
    datastore: &Arc<Mutex<DatastoreManager>>,
    commit_entry: &Value,
) -> Result<usize> {
    let mut stored = 0usize;
    for (source_contract, through_commit) in prefix_cert_sources(commit_entry) {
        let mut req = serde_json::json!({ "through_commit": through_commit });
        if let Some(contract) = &source_contract {
            req["source_contract"] = serde_json::json!(contract);
        }
        let resp = request_json(swarm, reqres_txs, peer_addr, "/contract/prefix_certs", Some(req))
            .await?;
        let certs = resp
            .get("certs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mgr = datastore.lock().await;
        let validators = mgr.validators()?;
        for cert in certs {
            if !cert_is_for(&cert, source_contract.as_deref(), &through_commit)
                || modality_validator::prefix_cert::cheap_include_prefix_cert(&cert, &validators)
                    .is_err()
            {
                continue;
            }
            let (Some(contract), Some(signer)) = (
                cert.get("source_contract").and_then(|v| v.as_str()),
                cert.get("validator_peer_id").and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            if mgr.has_prefix_cert_from(contract, &through_commit, signer)? {
                continue;
            }
            mgr.save_prefix_cert(&cert)?;
            stored += 1;
        }
    }
    Ok(stored)
}

fn cert_is_for(cert: &Value, source_contract: Option<&str>, through_commit: &str) -> bool {
    cert.get("through_commit").and_then(|v| v.as_str()) == Some(through_commit)
        && source_contract.is_none_or(|contract| {
            cert.get("source_contract").and_then(|v| v.as_str()) == Some(contract)
        })
}

async fn request_json(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    reqres_txs: &Arc<
        Mutex<
            HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
    peer_addr: &Multiaddr,
    path: &str,
    data: Option<Value>,
) -> Result<Value> {
    let Some(Protocol::P2p(target_peer_id)) = peer_addr.iter().last() else {
        anyhow::bail!("bootstrapper missing /p2p peer id");
    };
    let request = reqres::Request {
        path: path.to_string(),
        data,
    };
    let request_id = {
        let mut swarm = swarm.lock().await;
        swarm
            .behaviour_mut()
            .reqres
            .send_request(&target_peer_id, request)
    };
    let response = tokio::time::timeout(
        Duration::from_secs(REQRES_TIMEOUT_SECS),
        wait_for_reqres_response(reqres_txs, request_id),
    )
    .await
    .map_err(|_| anyhow::anyhow!("timeout requesting {path}"))??;
    if !response.ok {
        anyhow::bail!(
            "{path} failed: {}",
            response
                .errors
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".into())
        );
    }
    Ok(response.data.unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefix_cert_sources_name_recv_and_repost_sources() {
        let entry = json!({
            "body": [
                { "method": "post", "path": "/a.text", "value": "x" },
                { "method": "recv", "value": { "send_commit_id": "s1", "send_index": 1 } },
                {
                    "method": "repost",
                    "path": "/reposts/src/a.text",
                    "value": "x",
                    "source_contract": "src",
                    "source_path": "/a.text",
                    "source_commit": "c9"
                }
            ]
        });
        let sources = prefix_cert_sources(&entry);
        assert!(sources.contains(&(None, "s1".to_string())));
        assert!(sources.contains(&(Some("src".to_string()), "c9".to_string())));
    }

    #[test]
    fn a_cert_for_another_commit_is_not_taken() {
        let cert = json!({ "source_contract": "src", "through_commit": "c1" });
        assert!(cert_is_for(&cert, None, "c1"));
        assert!(cert_is_for(&cert, Some("src"), "c1"));
        assert!(!cert_is_for(&cert, Some("other"), "c1"));
        assert!(!cert_is_for(&cert, None, "c2"));
    }
}
