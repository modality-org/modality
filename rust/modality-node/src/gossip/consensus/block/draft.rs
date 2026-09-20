use anyhow::anyhow;
use anyhow::Result;
use serde_json;
use tokio::sync::mpsc;

use modality_datastore::models::ValidatorBlock;
use modality_datastore::DatastoreManager;
use modality_datastore::Model;
use modality_validator_consensus::communication::Message as ConsensusMessage;

pub const TOPIC: &str = "/consensus/block/draft";

pub async fn handler(
    data: String,
    _datastore_manager: &mut DatastoreManager,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<()> {
    let block_data =
        serde_json::from_str::<serde_json::Value>(&data).unwrap_or(serde_json::Value::Null);
    let block = ValidatorBlock::from_json_string(&data.clone())?;
    let from = block_data
        .get("peer_id")
        .ok_or_else(|| anyhow!("Missing peer_id field"))?
        .as_str()
        .ok_or_else(|| anyhow!("peer_id is not a string"))?;

    let msg = ConsensusMessage::DraftValidatorBlock {
        from: from.to_string(),
        to: String::new(),
        block,
    };
    consensus_tx.send(msg).await?;
    Ok(())
}
