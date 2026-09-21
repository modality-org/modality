//! Pull sequenced contract logs onto an observer (or any non-sequencer).
//!
//! Live path: certified validator-block gossip, then the same apply as sequencers.
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
use modality_validator_consensus::communication::Message as ConsensusMessage;

use crate::actions::validator::consensus::apply_certified_contract_events;
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
            if let ConsensusMessage::CertifiedValidatorBlock { block, .. } = msg {
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
                Err(_) => next.push(commit_entry),
            }
        }
        remaining = next;
    }
    Ok(applied)
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
