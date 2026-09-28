//! Pull certified sequencer blocks from committee peers.
//!
//! Gossip drops certificates. Each sequencer also asks every committee peer
//! for the certified blocks that peer authored, and hands them to the Shoal
//! loop through the same receive path as gossip.

use anyhow::Result;
use libp2p::request_response::OutboundRequestId;
use libp2p_identity::PeerId;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

use modality_datastore::models::SequencerBlock;
use modality_datastore::{DatastoreManager, Store};
use modality_sequencer_consensus::communication::Message as ConsensusMessage;

use super::consensus::SequencingControl;
use crate::reqres;
use crate::reqres::consensus::block::certified_since;
use crate::swarm::NodeSwarm;

const PULL_INTERVAL_SECS: u64 = 5;
const PULL_TIMEOUT_SECS: u64 = 10;

const PULLED_PREFIX: &str = "cert_sync/pulled";
const LAST_CERT_PREFIX: &str = "cert_sync/last_cert";

pub type ReqresTxs = Arc<Mutex<HashMap<OutboundRequestId, oneshot::Sender<reqres::Response>>>>;

fn read_u64(mgr: &DatastoreManager, key: &str) -> Option<u64> {
    let data = mgr.node_state().get(key).ok()??;
    String::from_utf8(data).ok()?.parse().ok()
}

fn write_u64(mgr: &DatastoreManager, key: &str, value: u64) {
    if let Err(e) = mgr.node_state().put(key, value.to_string().as_bytes()) {
        log::warn!("Failed to write {}: {}", key, e);
    }
}

/// Remember the highest certified round seen from `author`.
pub fn record_cert_round(mgr: &DatastoreManager, author: &str, round: u64) {
    let key = format!("{LAST_CERT_PREFIX}/{author}");
    if read_u64(mgr, &key).is_none_or(|prev| round > prev) {
        write_u64(mgr, &key, round);
    }
}

/// Highest certified round seen per block author.
pub fn last_cert_rounds(mgr: &DatastoreManager) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for item in mgr.node_state().iterator(LAST_CERT_PREFIX) {
        let Ok((key, value)) = item else {
            continue;
        };
        let Ok(key) = String::from_utf8(key.to_vec()) else {
            continue;
        };
        let Some(author) = key
            .strip_prefix(LAST_CERT_PREFIX)
            .and_then(|k| k.strip_prefix('/'))
        else {
            continue;
        };
        if let Some(round) = String::from_utf8(value.to_vec())
            .ok()
            .and_then(|s| s.parse().ok())
        {
            out.push((author.to_string(), round));
        }
    }
    out.sort();
    out
}

/// Every few seconds, pull each committee peer's certified blocks we have not seen.
pub fn start_pull(
    own_peer_id: String,
    swarm: Arc<Mutex<NodeSwarm>>,
    reqres_txs: ReqresTxs,
    datastore: Arc<Mutex<DatastoreManager>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    control: SequencingControl,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PULL_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if !control.participate.load(Ordering::Relaxed) {
                continue;
            }
            let committee = control.committee();
            if !committee.contains(&own_peer_id) {
                continue;
            }
            for peer in committee.iter().filter(|p| **p != own_peer_id) {
                match pull_from_peer(peer, &swarm, &reqres_txs, &datastore, &consensus_tx).await {
                    Ok(0) => {}
                    Ok(n) => log::info!(
                        "Pulled {} certified block(s) from {}",
                        n,
                        &peer[..16.min(peer.len())]
                    ),
                    Err(e) => log::debug!(
                        "Certified-block pull from {} failed: {}",
                        &peer[..16.min(peer.len())],
                        e
                    ),
                }
            }
        }
    });
}

async fn pull_from_peer(
    peer: &str,
    swarm: &Arc<Mutex<NodeSwarm>>,
    reqres_txs: &ReqresTxs,
    datastore: &Arc<Mutex<DatastoreManager>>,
    consensus_tx: &mpsc::Sender<ConsensusMessage>,
) -> Result<usize> {
    let peer_id = PeerId::from_str(peer)?;
    let pulled_key = format!("{PULLED_PREFIX}/{peer}");
    let since = {
        let mgr = datastore.lock().await;
        read_u64(&mgr, &pulled_key).unwrap_or(0)
    };

    let request = reqres::Request {
        path: certified_since::PATH.to_string(),
        data: Some(json!({
            "author": peer,
            "since_round": since,
            "limit": certified_since::MAX_BLOCKS,
        })),
    };
    let (tx, rx) = oneshot::channel();
    let request_id = {
        let mut swarm = swarm.lock().await;
        let request_id = swarm.behaviour_mut().reqres.send_request(&peer_id, request);
        // Register before releasing the swarm; the networking task routes the
        // response only while holding it.
        reqres_txs.lock().await.insert(request_id, tx);
        request_id
    };
    let response = match tokio::time::timeout(Duration::from_secs(PULL_TIMEOUT_SECS), rx).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => anyhow::bail!("response channel closed"),
        Err(_) => {
            reqres_txs.lock().await.remove(&request_id);
            anyhow::bail!("timed out");
        }
    };
    if !response.ok {
        anyhow::bail!(
            "{}",
            response
                .errors
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown error".into())
        );
    }
    let data = response.data.unwrap_or_default();
    let current_round = data
        .get("current_round")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let scanned_through = data
        .get("scanned_through")
        .and_then(|v| v.as_u64())
        .unwrap_or(since);
    let blocks: Vec<SequencerBlock> = data
        .get("blocks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|b| serde_json::from_value(b.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let count = blocks.len();
    for block in blocks {
        consensus_tx
            .send(ConsensusMessage::CertifiedSequencerBlock {
                from: peer.to_string(),
                to: String::new(),
                block,
            })
            .await?;
    }

    // A peer whose round is below our mark lost its round counter; rescan it.
    let next = if current_round < since {
        0
    } else {
        scanned_through
    };
    if next != since {
        let mgr = datastore.lock().await;
        write_u64(&mgr, &pulled_key, next);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_highest_cert_round_per_author() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        record_cert_round(&mgr, "peer-b", 7);
        record_cert_round(&mgr, "peer-a", 3);
        record_cert_round(&mgr, "peer-b", 5);
        record_cert_round(&mgr, "peer-a", 4);
        assert_eq!(
            last_cert_rounds(&mgr),
            vec![("peer-a".to_string(), 4), ("peer-b".to_string(), 7)]
        );
    }
}
