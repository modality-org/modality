use anyhow::Result;
use log::warn;
use std::sync::Arc;

use crate::communication::Communication;
use crate::sequencing::Sequencing;

use modality_network_datastore::{Model, NetworkDatastore};
use modality_network_datastore::models::{Page, Round, RoundMessage};
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
    latest_seen_at_round: Option<u64>,
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
            latest_seen_at_round: None,
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

    async fn get_scribes_at_round(&self, round: u64) -> Result<Vec<String>> {
        self.sequencing.get_scribes_at_round(round).await
    }

    async fn consensus_threshold_for_round(&self, round: u64) -> Result<u64> {
        self.sequencing.consensus_threshold_for_round(round).await
    }

    pub async fn on_receive_draft_page(&mut self, page_data: serde_json::Value) -> Result<Option<Ack>> {
        let page = Page::create_from_json(page_data.clone())?;
        if !page.validate_sig()? {
            warn!("invalid sig");
            return Ok(None);
        }

        let round_scribes = self.get_scribes_at_round(page.round.try_into().unwrap()).await?;
        if !round_scribes.contains(&page.scribe) {
            warn!("ignoring non-scribe {} at round {}", page.scribe, page.round);
            return Ok(None);
        }

        let current_round = self.datastore.get_current_round().await?;

        if page.round > current_round {
            self.on_receive_draft_page_from_later_round(page_data).await
        } else if page.round < current_round {
            self.on_receive_draft_page_from_earlier_round(page_data).await
        } else {
            self.on_receive_draft_page_from_current_round(page_data).await
        }
    }

    async fn on_receive_draft_page_from_earlier_round(
        &self,
        page_data: serde_json::Value,
    ) -> Result<Option<Ack>> {
        let current_round = self.datastore.get_current_round().await?;
        let page = Page::create_from_json(page_data)?;

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = page.generate_late_ack(keypair, current_round)?;
            if let Some(communication) = &self.communication {                
                communication.send_page_late_ack(
                    &peerid.clone(),
                    &ack.scribe.clone(),
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
        let current_round = self.datastore.get_current_round().await?;
        let page = Page::create_from_json(page_data.clone())?;

        let round_message = RoundMessage::create_from_json(serde_json::json!({
            "round": page.round,
            "scribe": page.scribe,
            "type": "draft",
            "seen_at_round": current_round,
            "content": page_data
        }))?;
        round_message.save(&self.datastore).await?;

        if current_round < page.round {
            if self.latest_seen_at_round.is_none() || page.round > self.latest_seen_at_round.unwrap() {
                self.latest_seen_at_round = Some(page.round);
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
                    &ack.scribe.clone(),
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
        if whoami != ack.scribe {
            return Ok(());
        }

        let round = self.datastore.get_current_round().await?;
        if ack.round != round {
            return Ok(());
        }

        let round_scribes = self.get_scribes_at_round(ack.round).await?;
        if !round_scribes.contains(&ack.acker) {
            warn!("ignoring non-scribe ack {} at round {}", ack.acker, ack.round);
            return Ok(());
        }

        if let Some(mut page) = Page::find_one(
            &self.datastore,
            std::collections::HashMap::from([
                (String::from("round"), round.to_string()),
                (String::from("scribe"), whoami.to_string()),
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
        if !page.validate_sig()? {
            return Ok(None);
        }

        let round = self.datastore.get_current_round().await?;
        if page.round > round {
            self.on_receive_certified_page_from_later_round(page_data).await
        } else {
            self.on_receive_certified_page_from_current_round(page_data).await
        }
    }

    async fn on_receive_certified_page_from_later_round(
        &mut self,
        page_data: serde_json::Value,
    ) -> Result<Option<Page>> {
        let current_round = self.datastore.get_current_round().await?;
        let page = Page::from_json_object(page_data.clone())?;

        RoundMessage::from_json_object(serde_json::json!({
            "round": page.round,
            "scribe": page.scribe,
            "type": "certified",
            "seen_at_round": current_round,
            "content": page_data
        }))?
        .save(&self.datastore)
        .await?;

        if current_round < page.round {
            if self.latest_seen_at_round.is_none() || page.round > self.latest_seen_at_round.unwrap() {
                self.latest_seen_at_round = Some(page.round);
            }
        }
        Ok(None)
    }

    async fn on_receive_certified_page_from_current_round(
        &self,
        page_data: serde_json::Value,
    ) -> Result<Option<Page>> {
        let page = Page::from_json_object(page_data)?;
        if !page.validate_sig()? {
            return Ok(None);
        }
        let round = page.round;

        let last_round_threshold = self.consensus_threshold_for_round(round - 1).await?;
        let current_round_threshold = self.consensus_threshold_for_round(round).await?;

        let page_last_round_cert_count = page.last_round_certs.len() as u64;
        if round > 1 && (page_last_round_cert_count < last_round_threshold) {
            return Ok(None);
        }

        let has_valid_cert = page.validate_cert(current_round_threshold as usize)?;
        
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