use anyhow::Result;
use log::warn;
use std::sync::Arc;

use crate::communication::Communication;
use crate::sequencing::Sequencing;

use modality_network_datastore::{Model, NetworkDatastore};
use modality_network_datastore::models::{Page, Block, BlockMessage};
use modality_network_datastore::models::page::{Ack};
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
    intra_round_wait_time_ms: Option<u64>,
    no_events_round_wait_time_ms: Option<u64>,
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

    async fn get_scribes_at_block_id(&self, block_id: u64) -> Result<Vec<String>> {
        self.sequencing.get_scribes_at_block_id(block_id).await
    }

    async fn consensus_threshold_at_block_id(&self, block_id: u64) -> Result<u64> {
        self.sequencing.consensus_threshold_at_block_id(block_id).await
    }

    pub async fn on_receive_draft_page(&mut self, page_data: serde_json::Value) -> Result<Option<Ack>> {
        let page = Page::create_from_json(page_data.clone())?;
        if !page.validate_sigs()? {
            warn!("invalid sig");
            return Ok(None);
        }

        let round_scribes = self.get_scribes_at_block_id(page.block_id.try_into().unwrap()).await?;
        if !round_scribes.contains(&page.peer_id) {
            warn!("ignoring non-scribe {} at block_id {}", page.peer_id, page.block_id);
            return Ok(None);
        }

        let current_block_id = self.datastore.get_current_block_id().await?;

        if page.block_id > current_block_id {
            self.on_receive_draft_page_from_later_round(page_data).await
        } else if page.block_id < current_block_id {
            self.on_receive_draft_page_from_earlier_round(page_data).await
        } else {
            self.on_receive_draft_page_from_current_round(page_data).await
        }
    }

    async fn on_receive_draft_page_from_earlier_round(
        &self,
        page_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let current_block_id = self.datastore.get_current_block_id().await?;
        let page = Page::create_from_json(page_data)?;

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = page.generate_late_ack(keypair, current_block_id)?;
            if let Some(communication) = &self.communication {                
                communication.send_page_late_ack(
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

    async fn on_receive_draft_page_from_later_round(
        &mut self,
        page_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let current_block_id = self.datastore.get_current_block_id().await?;
        let page = Page::create_from_json(page_data.clone())?;

        let round_message = BlockMessage::create_from_json(serde_json::json!({
            "block_id": page.block_id,
            "peer_id": page.peer_id,
            "type": "draft",
            "seen_at_block_id": current_block_id,
            "content": page_data
        }))?;
        round_message.save(&self.datastore).await?;

        if current_block_id < page.block_id {
            if self.latest_seen_at_block_id.is_none() || page.block_id > self.latest_seen_at_block_id.unwrap() {
                self.latest_seen_at_block_id = Some(page.block_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_draft_page_from_current_round(
        &self,
        page_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let page = Page::create_from_json(page_data)?;

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = page.generate_ack(keypair)?;
            if let Some(communication) = &self.communication {
                communication.send_page_ack(
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

    pub async fn on_receive_page_ack(&self, ack: Option<Ack>) -> Result<()> {
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

        let block_id = self.datastore.get_current_block_id().await?;
        if ack.block_id != block_id {
            return Ok(());
        }

        let round_scribes = self.get_scribes_at_block_id(ack.block_id).await?;
        if !round_scribes.contains(&ack.acker) {
            warn!("ignoring non-scribe ack {} at block_id {}", ack.acker, ack.block_id);
            return Ok(());
        }

        if let Some(mut page) = Page::find_one(
            &self.datastore,
            std::collections::HashMap::from([
                (String::from("block_id"), block_id.to_string()),
                (String::from("peer_id"), whoami.to_string()),
            ]),
        )
        .await?
        {
            page.add_ack(ack)?;
            page.save(&self.datastore).await?;
        }

        Ok(())
    }

    pub async fn on_receive_certified_page(&mut self, page_data: serde_json::Value) -> Result<Option<Page>> {
        let page = Page::from_json_object(page_data.clone())?;
        if !page.validate_sigs()? {
            return Ok(None);
        }

        let block_id = self.datastore.get_current_block_id().await?;
        if page.block_id > block_id {
            self.on_receive_certified_page_from_later_round(page_data).await
        } else {
            self.on_receive_certified_page_from_current_round(page_data).await
        }
    }

    async fn on_receive_certified_page_from_later_round(
        &mut self,
        page_data: serde_json::Value,
    ) -> Result<Option<Page>> {
        let current_block_id = self.datastore.get_current_block_id().await?;
        let page = Page::from_json_object(page_data.clone())?;

        BlockMessage::from_json_object(serde_json::json!({
            "block_id": page.block_id,
            "peer_id": page.peer_id,
            "type": "certified",
            "seen_at_block_id": current_block_id,
            "content": page_data
        }))?
        .save(&self.datastore)
        .await?;

        if current_block_id < page.block_id {
            if self.latest_seen_at_block_id.is_none() || page.block_id > self.latest_seen_at_block_id.unwrap() {
                self.latest_seen_at_block_id = Some(page.block_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_certified_page_from_current_round(
        &self,
        page_data: serde_json::Value,
    ) -> Result<Option<Page>> {
        let page = Page::from_json_object(page_data)?;
        if !page.validate_sigs()? {
            return Ok(None);
        }
        let block_id = page.block_id;

        let last_block_threshold = self.consensus_threshold_at_block_id(block_id - 1).await?;
        let current_block_threshold = self.consensus_threshold_at_block_id(block_id).await?;

        let page_last_block_cert_count = page.prev_block_certs.len() as u64;
        if block_id > 1 && (page_last_block_cert_count < last_block_threshold) {
            return Ok(None);
        }

        let has_valid_cert = page.validate_cert(current_block_threshold as usize)?;
        
        if !has_valid_cert {
            return Ok(None);
        }

        page.save(&self.datastore).await?;
        Ok(Some(page))
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