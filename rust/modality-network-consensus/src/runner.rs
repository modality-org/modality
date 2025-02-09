use anyhow::Result;
use log::warn;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;

use crate::communication::Communication;
use crate::sequencing::Sequencing;

use modality_network_datastore::{Model, NetworkDatastore};
use modality_network_datastore::models::{Block, BlockMessage, Transaction};
use modality_network_datastore::models::block::Ack;
use modality_utils::keypair::Keypair;

// const INTRA_ROUND_WAIT_TIME_MS: u64 = 50;
// const NO_EVENTS_ROUND_WAIT_TIME_MS: u64 = 15000;
// const NO_EVENTS_POLL_WAIT_TIME_MS: u64 = 500;

#[derive(Clone)]
pub struct Runner {
    pub datastore: Arc<NetworkDatastore>,
    pub peerid: Option<String>,
    pub communication: Option<Arc<Mutex<dyn Communication>>>,
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
        communication: Option<Arc<Mutex<dyn Communication>>>,
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
                let mut comm = communication.lock().await;
                comm.send_block_late_ack(
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
                let mut comm = communication.lock().await;
                comm.send_block_ack(
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

    pub async fn speed_up_to_latest_uncertified_round(&mut self) -> Result<()> {
        let mut round_certified = true;
        let mut round = self.datastore.get_current_round().await? + 1;

        while round_certified {
            let prev_round_certs = self.get_or_fetch_prev_round_certs(round).await?;
            let existing_certs = BlockMessage::find_all_in_round_of_type(
                &self.datastore,
                round - 1,
                "certified",
            ).await?;

            for draft in existing_certs {
                let draft_content = draft.content.clone();
                self.datastore.delete(&draft.get_id()).await?;
                self.on_receive_certified_block(draft_content).await?;
            }

            let threshold = self.consensus_threshold_at_round_id(round - 1).await?;
            let cert_count = prev_round_certs.len() as u64;

            if cert_count > 0 && threshold > 0 && cert_count >= threshold {
                round += 1;
            } else {
                round_certified = false;
            }
        }

        let newest_uncertified_round = round - 1;
        self.datastore.set_current_round(newest_uncertified_round).await?;
        Ok(())
    }

    pub async fn get_or_fetch_prev_round_certs(&self, round: u64) -> Result<HashMap<String, String>> {
        if round == 0 {
            return Ok(HashMap::new());
        }

        let prev_round = round - 1;
        let mut prev_round_certs = self.datastore.get_timely_certs_at_round(prev_round).await?;
        let prev_round_scribes = self.get_scribes_at_round_id(prev_round).await?;
        let threshold = self.consensus_threshold_at_round_id(prev_round).await?;

        if prev_round_certs.len() as u64 >= threshold {
            return Ok(prev_round_certs);
        }

        if let Some(communication) = &self.communication {
            for peer_id in &prev_round_scribes {
                let mut block_data = communication.lock().await
                    .fetch_scribe_round_certified_block(
                        self.peerid.as_ref().unwrap(),
                        peer_id,
                        peer_id,
                        round,
                    ).await?;

                if block_data.is_none() {
                    for alt_peer_id in &prev_round_scribes {
                        block_data = communication.lock().await
                            .fetch_scribe_round_certified_block(
                                self.peerid.as_ref().unwrap(),
                                alt_peer_id,
                                peer_id,
                                round,
                            ).await?;

                        if block_data.is_some() {
                            break;
                        }
                    }
                }

                if let Some(block_data) = block_data {
                    let block = Block::from_json_object(block_data.to_json_object()?)?;
                    if block.validate_cert(threshold as usize)? {
                        block.save(&self.datastore).await?;
                    }
                }
            }
        }

        prev_round_certs = self.datastore.get_timely_certs_at_round(prev_round).await?;
        Ok(prev_round_certs)
    }

    pub async fn run_round(&mut self, signal: Option<CancellationToken>) -> Result<()> {
        self.speed_up_to_latest_uncertified_round().await?;
        let mut round = self.datastore.get_current_round().await?;

        let mut working_round = round;

        while false {
            if working_round < 1 {
                break;
            }

            let prev_round_certs = self.get_or_fetch_prev_round_certs(working_round).await?;
            let threshold = self.consensus_threshold_at_round_id(working_round - 1).await?;
            let cert_count = prev_round_certs.len() as u64;

            if cert_count >= threshold {
                break;
            } else {
                warn!("NOT ENOUGH {}/{} going back to round {}", cert_count, threshold, working_round - 1);
                working_round -= 1;
            }
        }

        let prev_round_certs = self.get_or_fetch_prev_round_certs(round).await?;
        let threshold = self.consensus_threshold_at_round_id(round - 1).await?;
        let cert_count = prev_round_certs.len() as u64;

        if cert_count < threshold {
            warn!("prev_round: {}, cert_count: {}, threshold: {}, prev_round_certs: {:?}",
                round - 1, cert_count, threshold, prev_round_certs);
            return Err(anyhow::anyhow!("not enough certs to start round"));
        }

        let current_round_threshold = self.consensus_threshold_at_round_id(round).await?;
        let existing_this_round_certs = BlockMessage::find_all_in_round_of_type(
            &self.datastore,
            round,
            "certified",
        ).await?;

        if existing_this_round_certs.len() as u64 >= current_round_threshold {
            self.bump_current_round().await?;
            round = self.datastore.get_current_round().await?;
        }

        let mut cc_events = Transaction::find_all(&self.datastore).await?;
        let mut keep_waiting_for_events = cc_events.is_empty();

        if keep_waiting_for_events {
            if let Some(wait_time) = self.no_events_round_wait_time_ms {
                tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            }
        }

        while keep_waiting_for_events {
            if let Some(wait_time) = self.no_events_poll_wait_time_ms {
                tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            }
            cc_events = Transaction::find_all(&self.datastore).await?;
            if !cc_events.is_empty() {
                keep_waiting_for_events = false;
            }
        }

        let mut events = Vec::new();
        for cc_event in cc_events {
            events.push(serde_json::json!({
                "contract_id": cc_event.contract_id,
                "commit_id": cc_event.commit_id,
            }));
            cc_event.delete(&self.datastore).await?;
        }

        let mut block = Block::create_from_json(serde_json::json!({
            "round_id": round,
            "peer_id": self.peerid.clone(),
            "prev_round_certs": prev_round_certs,
            "events": []
        }))?;
        if let Some(keypair) = &self.keypair {
            block.generate_sigs(keypair)?;
        }
        block.save(&self.datastore).await?;

        if let Some(communication) = &self.communication {
            let block_data = block.clone();
            communication.lock().await.broadcast_draft_block(
                &self.peerid.clone().unwrap(),
                &block_data,
            ).await?;
        }

        // Handle enqueued round messages
        let existing_drafts = BlockMessage::find_all_in_round_of_type(
            &self.datastore,
            round,
            "draft",
        ).await?;

        for draft in existing_drafts {
            let draft_content = draft.content.clone();
            self.datastore.delete(&draft.get_id()).await?;
            self.on_receive_draft_block(draft_content).await?;
        }

        let mut keep_waiting_for_acks = self.latest_seen_at_block_id.is_none();
        let mut keep_waiting_for_certs = true;

        while keep_waiting_for_acks || keep_waiting_for_certs {
            if let Some(latest_seen) = self.latest_seen_at_block_id {
                if latest_seen > round {
                    self.jump_to_round(latest_seen).await?;
                    self.latest_seen_at_block_id = None;
                    return Ok(());
                }
            }

            if signal.as_ref().map_or(false, |s| s.is_cancelled()) {
                return Err(anyhow::anyhow!("aborted"));
            }

            if keep_waiting_for_acks {
                block.reload(&self.datastore).await?;
                let valid_acks = block.count_valid_acks()?;

                if valid_acks >= (current_round_threshold as usize) {
                    if let Some(keypair) = &self.keypair {
                        block.generate_cert(keypair)?;
                    }
                    block.save(&self.datastore).await?;

                    if let Some(communication) = &self.communication {
                        communication.lock().await.broadcast_certified_block(
                            &self.peerid.clone().unwrap(),
                            &block.clone(),
                        ).await?;
                    }
                    keep_waiting_for_acks = false;
                }
            }

            if keep_waiting_for_certs {
                let current_round_certs = self.datastore.get_timely_certs_at_round(round).await?;
                if current_round_certs.len() as u64 >= current_round_threshold {
                    keep_waiting_for_certs = false;
                }
            }

            if let Some(wait_time) = self.intra_round_wait_time_ms {
                tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            } else {
                tokio::task::yield_now().await;
            }
        }

        self.bump_current_round().await?;
        Ok(())
    }

    pub async fn jump_to_round(&mut self, round_num: u64) -> Result<()> {
        let current_round_num = self.datastore.get_current_round().await?;
        for _i in (current_round_num + 1)..round_num {
            // TODO: Maybe handle jumping from earlier rounds
        }
        self.datastore.set_current_round(round_num).await?;
        Ok(())
    }

    pub async fn bump_current_round(&mut self) -> Result<()> {
        self.datastore.bump_current_round().await?;
        Ok(())
    }

    pub async fn run_until_round(&mut self, target_round: u64, signal: Option<CancellationToken>) -> Result<()> {
        let mut current_round = self.datastore.get_current_round().await?;
        while current_round < target_round {
            if signal.as_ref().map_or(false, |s| s.is_cancelled()) {
                return Err(anyhow::anyhow!("aborted"));
            }
            self.run_round(signal.clone()).await?;
            current_round = self.datastore.get_current_round().await?;
        }
        Ok(())
    }

    pub async fn run(
        &mut self,
        signal: Option<CancellationToken>,
        before_each_round: Option<Box<dyn Fn() -> Result<()>>>,
        after_each_round: Option<Box<dyn Fn() -> Result<()>>>,
    ) -> Result<()> {
        loop {
            if let Some(before) = &before_each_round {
                before()?;
            }
            self.run_round(signal.clone()).await?;
            if let Some(after) = &after_each_round {
                after()?;
            }
        }
    }
}

pub struct RunnerProps {
    pub datastore: Arc<NetworkDatastore>,
    pub peerid: Option<String>,
    pub keypair: Option<Arc<Keypair>>,
    pub communication: Option<Arc<Mutex<dyn Communication>>>,
    pub sequencing: Arc<dyn Sequencing>,
}