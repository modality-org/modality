use anyhow::anyhow;
use anyhow::Result;
use serde_json;
use tokio::sync::mpsc;

use modality_datastore::models::validator::block::Ack;
use modality_datastore::DatastoreManager;
use modality_validator_consensus::communication::Message as ConsensusMessage;

use crate::reqres::Response;

pub async fn handler(
    data: Option<serde_json::Value>,
    _datastore_manager: &DatastoreManager,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<Response> {
    log::info!("REQ /data/block/ack {:?}", data);
    let response = Response {
        ok: true,
        data: None,
        errors: None,
    };

    let ack_data = data.ok_or_else(|| anyhow!("Missing ack data"))?;

    let peer_id = ack_data
        .get("peer_id")
        .ok_or_else(|| anyhow!("Missing peer_id"))?
        .as_str()
        .ok_or_else(|| anyhow!("peer_id must be a string"))?
        .to_string();

    let round_id = ack_data
        .get("round_id")
        .ok_or_else(|| anyhow!("Missing round_id"))?
        .as_u64()
        .ok_or_else(|| anyhow!("round_id must be a number"))?
        .to_string();

    let closing_sig = ack_data
        .get("closing_sig")
        .ok_or_else(|| anyhow!("Missing closing_sig"))?
        .as_str()
        .ok_or_else(|| anyhow!("closing_sig must be a string"))?
        .to_string();

    let acker = ack_data
        .get("acker")
        .ok_or_else(|| anyhow!("Missing acker"))?
        .as_str()
        .ok_or_else(|| anyhow!("acker must be a string"))?
        .to_string();

    let acker_sig = ack_data
        .get("acker_sig")
        .ok_or_else(|| anyhow!("Missing acker_sig"))?
        .as_str()
        .ok_or_else(|| anyhow!("acker_sig must be a string"))?
        .to_string();

    let round_id = round_id
        .parse::<u64>()
        .map_err(|_| anyhow!("round_id must be a valid u64"))?;
    let ack = Ack {
        peer_id: peer_id.clone(),
        round_id,
        closing_sig,
        acker: acker.clone(),
        acker_sig,
    };

    let msg = ConsensusMessage::ValidatorBlockAck {
        from: acker,
        to: peer_id,
        ack,
    };
    // The networking task calls this while holding the datastore lock.
    // Waiting on the bounded consensus channel deadlocks the Shoal loop,
    // which is that channel's consumer and also needs the lock.
    match consensus_tx.try_send(msg) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            log::warn!("consensus channel full; dropping block ack so the node can keep serving");
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": "consensus channel full"})),
            });
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            anyhow::bail!("consensus channel closed");
        }
    }

    Ok(response)
}
