use anyhow::Result;
use log::warn;
use std::sync::Arc;

use crate::communication::Communication;
use crate::sequencing::Sequencing;

use modality_network_datastore::{Model, NetworkDatastore};
use modality_network_datastore::models::{Block, BlockMessage};
use modality_network_datastore::models::block::{Ack};
use modality_utils::keypair::Keypair;

// const INTRA_ROUND_WAIT_TIME_MS: u64 = 50;
// const NO_EVENTS_ROUND_WAIT_TIME_MS: u64 = 15000;
// const NO_EVENTS_POLL_WAIT_TIME_MS: u64 = 500;

#[derive(Clone)]
pub struct Runner {
    pub datastore: Arc<NetworkDatastore>,
    pub peerid: Option<String>,
    pub communication: Option<Arc<dyn Communication>>,
    keypair: Option<Arc<Keypair>>,
    sequencing: Arc<dyn Sequencing>,
    latest_seen_at_block_id: Option<u64>,
    #[allow(unused)]
    intra_round_wait_time_ms: Option<u64>,
    #[allow(unused)]
    no_events_round_wait_time_ms: Option<u64>,
    #[allow(unused)]
    no_events_poll_wait_time_ms: Option<u64>,
}

impl Runner {
    pub fn new(
        datastore: Arc<NetworkDatastore>,
        peerid: Option<String>,
        keypair: Option<Arc<Keypair>>,
        communication: Option<Arc<dyn Communication>>,
        sequencing: Arc<dyn Sequencing>,
    ) -> Self {
        Runner {
            datastore,
            peerid,
            keypair,
            communication,
            sequencing,
            latest_seen_at_block_id: None,
            intra_round_wait_time_ms: None,
            no_events_round_wait_time_ms: None,
            no_events_poll_wait_time_ms: None,
        }
    }

    pub fn create(props: RunnerProps) -> Self {
        Self::new(
            props.datastore,
            props.peerid,
            props.keypair,
            props.communication,
            props.sequencing,
        )
    }

    async fn get_scribes_at_round_id(&self, round_id: u64) -> Result<Vec<String>> {
        self.sequencing.get_scribes_at_round_id(round_id).await
    }

    async fn consensus_threshold_at_round_id(&self, round_id: u64) -> Result<u64> {
        self.sequencing.consensus_threshold_at_round_id(round_id).await
    }

    pub async fn on_receive_draft_block(&mut self, block_data: serde_json::Value) -> Result<Option<Ack>> {
        let block = Block::create_from_json(block_data.clone())?;
        if !block.validate_sigs()? {
            warn!("invalid sig");
            return Ok(None);
        }

        let round_scribes = self.get_scribes_at_round_id(block.round_id.try_into().unwrap()).await?;
        if !round_scribes.contains(&block.peer_id) {
            warn!("ignoring non-scribe {} at round_id {}", block.peer_id, block.round_id);
            return Ok(None);
        }

        let current_block_id = self.datastore.get_current_round().await?;

        if block.round_id > current_block_id {
            self.on_receive_draft_block_from_later_round(block_data).await
        } else if block.round_id < current_block_id {
            self.on_receive_draft_block_from_earlier_round(block_data).await
        } else {
            self.on_receive_draft_block_from_current_round(block_data).await
        }
    }

    async fn on_receive_draft_block_from_earlier_round(
        &self,
        block_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let current_block_id = self.datastore.get_current_round().await?;
        let block = Block::create_from_json(block_data)?;

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = block.generate_late_ack(keypair, current_block_id)?;
            if let Some(communication) = &self.communication {                
                communication.send_block_late_ack(
                    &peerid.clone(),
                    &ack.peer_id.clone(),
                    &ack.clone(),
                ).await?;
            }
            Ok(Some(ack))
        } else {
            Ok(None)
        }
    }

    async fn on_receive_draft_block_from_later_round(
        &mut self,
        block_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let current_block_id = self.datastore.get_current_round().await?;
        let block = Block::create_from_json(block_data.clone())?;

        let round_message = BlockMessage::create_from_json(serde_json::json!({
            "round_id": block.round_id,
            "peer_id": block.peer_id,
            "type": "draft",
            "seen_at_block_id": current_block_id,
            "content": block_data
        }))?;
        round_message.save(&self.datastore).await?;

        if current_block_id < block.round_id {
            if self.latest_seen_at_block_id.is_none() || block.round_id > self.latest_seen_at_block_id.unwrap() {
                self.latest_seen_at_block_id = Some(block.round_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_draft_block_from_current_round(
        &self,
        block_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let block = Block::create_from_json(block_data)?;

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = block.generate_ack(keypair)?;
            if let Some(communication) = &self.communication {
                communication.send_block_ack(
                    &peerid.clone(),
                    &ack.peer_id.clone(),
                    &ack.clone(),
                ).await?;
            }
            Ok(Some(ack))
        } else {
            Ok(None)
        }
    }

    pub async fn on_receive_block_ack(&self, ack: Option<Ack>) -> Result<()> {
        let Some(ack) = ack else {
            return Ok(());
        };

        let Some(keypair) = &self.keypair else {
            return Ok(());
        };

        let whoami = keypair.as_public_address();
        if whoami != ack.peer_id {
            return Ok(());
        }

        let round_id = self.datastore.get_current_round().await?;
        if ack.round_id != round_id {
            return Ok(());
        }

        let round_scribes = self.get_scribes_at_round_id(ack.round_id).await?;
        if !round_scribes.contains(&ack.acker) {
            warn!("ignoring non-scribe ack {} at round_id {}", ack.acker, ack.round_id);
            return Ok(());
        }

        if let Some(mut block) = Block::find_one(
            &self.datastore,
            std::collections::HashMap::from([
                (String::from("round_id"), round_id.to_string()),
                (String::from("peer_id"), whoami.to_string()),
            ]),
        )
        .await?
        {
            block.add_ack(ack)?;
            block.save(&self.datastore).await?;
        }

        Ok(())
    }

    pub async fn on_receive_certified_block(&mut self, block_data: serde_json::Value) -> Result<Option<Block>> {
        let block = Block::from_json_object(block_data.clone())?;
        if !block.validate_sigs()? {
            return Ok(None);
        }

        let round_id = self.datastore.get_current_round().await?;
        if block.round_id > round_id {
            self.on_receive_certified_block_from_later_round(block_data).await
        } else {
            self.on_receive_certified_block_from_current_round(block_data).await
        }
    }

    async fn on_receive_certified_block_from_later_round(
        &mut self,
        block_data: serde_json::Value,
    ) -> Result<Option<Block>> {
        let current_block_id = self.datastore.get_current_round().await?;
        let block = Block::from_json_object(block_data.clone())?;

        BlockMessage::from_json_object(serde_json::json!({
            "round_id": block.round_id,
            "peer_id": block.peer_id,
            "type": "certified",
            "seen_at_block_id": current_block_id,
            "content": block_data
        }))?
        .save(&self.datastore)
        .await?;

        if current_block_id < block.round_id {
            if self.latest_seen_at_block_id.is_none() || block.round_id > self.latest_seen_at_block_id.unwrap() {
                self.latest_seen_at_block_id = Some(block.round_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_certified_block_from_current_round(
        &self,
        block_data: serde_json::Value,
    ) -> Result<Option<Block>> {
        let block = Block::from_json_object(block_data)?;
        if !block.validate_sigs()? {
            return Ok(None);
        }
        let round_id = block.round_id;

        let last_block_threshold = self.consensus_threshold_at_round_id(round_id - 1).await?;
        let current_block_threshold = self.consensus_threshold_at_round_id(round_id).await?;

        let block_last_block_cert_count = block.prev_round_certs.len() as u64;
        if round_id > 1 && (block_last_block_cert_count < last_block_threshold) {
            return Ok(None);
        }

        let has_valid_cert = block.validate_cert(current_block_threshold as usize)?;
        
        if !has_valid_cert {
            return Ok(None);
        }

        block.save(&self.datastore).await?;
        Ok(Some(block))
    }

    // Additional methods can be added following the same pattern
}

pub struct RunnerProps {
    pub datastore: Arc<NetworkDatastore>,
    pub peerid: Option<String>,
    pub keypair: Option<Arc<Keypair>>,
    pub communication: Option<Arc<dyn Communication>>,
    pub sequencing: Arc<dyn Sequencing>,
}