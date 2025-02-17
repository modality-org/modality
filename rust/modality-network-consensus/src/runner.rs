use anyhow::Result;
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::communication::Communication;
use crate::election;
use crate::sequencing::static_authority::StaticAuthority;
use crate::sequencing::Sequencing;

use modality_network_datastore::models::block::Ack;
use modality_network_datastore::models::{Block, BlockMessage, Transaction};
use modality_network_datastore::{Model, NetworkDatastore};
use modality_utils::keypair::{self, Keypair};

// const INTRA_ROUND_WAIT_TIME_MS: u64 = 50;
// const NO_EVENTS_ROUND_WAIT_TIME_MS: u64 = 15000;
// const NO_EVENTS_POLL_WAIT_TIME_MS: u64 = 500;

#[async_trait::async_trait]
pub trait ConsensusRunner: Send + Sync {
    fn peerid(&self) -> &str;
    async fn on_receive_draft_block(&self, block_data: &Block) -> Result<Option<Ack>>;
    async fn on_receive_block_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_block_late_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_certified_block(&self, block_data: &Block) -> Result<Option<Block>>;
    async fn on_fetch_scribe_round_certified_block_request(
        &self,
        peer_id: &str,
        round_id: u64,
    ) -> Result<Option<Block>>;
}
#[derive(Clone)]
pub struct Runner {
    pub datastore: Arc<Mutex<NetworkDatastore>>,
    pub peerid: Option<String>,
    pub communication: Option<Arc<Mutex<dyn Communication>>>,
    keypair: Option<Keypair>,
    sequencing: Arc<dyn Sequencing>,
    latest_seen_at_block_id: Option<u64>,
    #[allow(unused)]
    intra_round_wait_time_ms: Option<u64>,
    #[allow(unused)]
    no_events_round_wait_time_ms: Option<u64>,
    #[allow(unused)]
    no_events_poll_wait_time_ms: Option<u64>,
}

#[async_trait::async_trait]
impl ConsensusRunner for Runner {
    fn peerid(&self) -> &str {
        self.peerid.as_ref().map(|id| id.as_str()).unwrap_or("")
    }

    async fn on_receive_draft_block(&self, block: &Block) -> Result<Option<Ack>> {
        if !block.validate_sigs()? {
            warn!("invalid sig");
            return Ok(None);
        }

        let round_scribes = self
            .get_scribes_at_round_id(block.round_id.try_into().unwrap())
            .await?;
        if !round_scribes.contains(&block.peer_id) {
            warn!(
                "ignoring non-scribe {} at round_id {}",
                block.peer_id, block.round_id
            );
            return Ok(None);
        }

        let current_round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };

        if block.round_id > current_round_id {
            self.on_receive_draft_block_from_later_round(block).await
        } else if block.round_id < current_round_id {
            self.on_receive_draft_block_from_earlier_round(block).await
        } else {
            self.on_receive_draft_block_from_current_round(block).await
        }
    }

    async fn on_receive_block_ack(&self, ack: &Ack) -> Result<()> {
        let Some(keypair) = &self.keypair else {
            return Ok(());
        };

        let whoami = keypair.as_public_address();
        // if whoami != ack.peer_id {
        //     return Ok(());
        // }

        let round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };
        if ack.round_id != round_id {
            return Ok(());
        }

        let round_scribes = self.get_scribes_at_round_id(ack.round_id).await?;
        if !round_scribes.contains(&ack.acker) {
            warn!(
                "ignoring non-scribe ack {} at round_id {}",
                ack.acker, ack.round_id
            );
            return Ok(());
        }

        {
            let datastore = self.datastore.lock().await;
            if let Some(mut block) = Block::find_one(
                &datastore,
                std::collections::HashMap::from([
                    (String::from("round_id"), round_id.to_string()),
                    (String::from("peer_id"), whoami.to_string()),
                ]),
            )
            .await?
            {
                block.add_ack(ack.clone())?;
                block.save(&datastore).await?;
                log::info!("ACKS {:?}", block.acks);
            }
        }
        Ok(())
    }

    async fn on_receive_block_late_ack(&self, _ack: &Ack) -> Result<()> {
        Ok(())
    }

    async fn on_receive_certified_block(&self, block: &Block) -> Result<Option<Block>> {
        if !block.validate_sigs()? {
            return Ok(None);
        }

        let round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };
        if block.round_id > round_id {
            return self
                .on_receive_certified_block_from_later_round(block)
                .await;
        } else {
            return self
                .on_receive_certified_block_from_current_round(block)
                .await;
        }
    }

    async fn on_fetch_scribe_round_certified_block_request(
        &self,
        peer_id: &str,
        round_id: u64,
    ) -> Result<Option<Block>> {
        // Search directly in the datastore instead of going through communication
        let block = {
            let datastore = self.datastore.lock().await;
            Block::find_one(
                &datastore,
                std::collections::HashMap::from([
                    (String::from("round_id"), round_id.to_string()),
                    (String::from("peer_id"), peer_id.to_string()),
                ]),
            )
            .await?
        };

        // Only return blocks that are certified
        if let Some(block) = block {
            let threshold = self.consensus_threshold_at_round_id(round_id).await?;
            if block.validate_cert(threshold as usize)? {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }
}

impl Runner {
    pub fn new(
        datastore: Arc<Mutex<NetworkDatastore>>,
        peerid: Option<String>,
        keypair: Option<Keypair>,
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
        self.sequencing
            .consensus_threshold_at_round_id(round_id)
            .await
    }

    async fn on_receive_draft_block_from_earlier_round(
        &self,
        block: &Block,
    ) -> Result<Option<Ack>> {
        let current_round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };

        if let (Some(peerid), Some(keypair)) = (&self.peerid, &self.keypair) {
            let ack = block.generate_late_ack(keypair, current_round_id)?;
            if let Some(communication) = &self.communication {
                let mut comm = communication.lock().await;
                comm.send_block_late_ack(&peerid.clone(), &ack.peer_id.clone(), &ack.clone())
                    .await?;
            }
            Ok(Some(ack))
        } else {
            Ok(None)
        }
    }

    async fn on_receive_draft_block_from_later_round(&self, block: &Block) -> Result<Option<Ack>> {
        let current_round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };

        let round_message = BlockMessage::create_from_json(serde_json::json!({
            "round_id": block.round_id,
            "peer_id": block.peer_id,
            "type": "draft",
            "seen_at_block_id": current_round_id,
            "content": block.to_draft_json_object()
        }))?;
        {
            let datastore = self.datastore.lock().await;
            round_message.save(&datastore).await?;
        }

        if current_round_id < block.round_id {
            if self.latest_seen_at_block_id.is_none()
                || block.round_id > self.latest_seen_at_block_id.unwrap()
            {
                // self.latest_seen_at_block_id = Some(block.round_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_draft_block_from_current_round(
        &self,
        block: &Block,
    ) -> Result<Option<Ack>> {
        let ack = match (&self.peerid, &self.keypair) {
            (Some(_peerid), Some(keypair)) => Some(block.generate_ack(keypair)?),
            _ => None,
        };

        if let Some(ack) = &ack {
            if let Some(communication) = &self.communication {
                let peerid = self.peerid.as_ref().unwrap().clone();
                let peer_id = ack.peer_id.clone();
                let ack_clone = ack.clone();

                // Get communication under short lock
                let mut comm = communication.lock().await;
                comm.send_block_ack(&peerid, &peer_id, &ack_clone).await?;
            }
        }

        Ok(ack)
    }

    async fn on_receive_certified_block_from_later_round(
        &self,
        block: &Block,
    ) -> Result<Option<Block>> {
        let current_round_id = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };

        {
            let datastore = self.datastore.lock().await;
            BlockMessage::from_json_object(serde_json::json!({
                "round_id": block.round_id,
                "peer_id": block.peer_id,
                "type": "certified",
                "seen_at_block_id": current_round_id,
                "content": block.to_draft_json_object()
            }))?
            .save(&datastore)
            .await?;
        }

        if current_round_id < block.round_id {
            if self.latest_seen_at_block_id.is_none()
                || block.round_id > self.latest_seen_at_block_id.unwrap()
            {
                // TODO
                // self.latest_seen_at_block_id = Some(block.round_id);
            }
        }
        Ok(None)
    }

    async fn on_receive_certified_block_from_current_round(
        &self,
        block: &Block,
    ) -> Result<Option<Block>> {
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

        {
            let datastore = self.datastore.lock().await;
            block.save(&datastore).await?;
        }
        Ok(Some(block.clone()))
    }

    pub async fn speed_up_to_latest_uncertified_round(&mut self) -> Result<()> {
        let mut round_certified = true;
        let mut round = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await? + 1
        };

        while round_certified {
            let prev_round_certs = self.get_or_fetch_prev_round_certs(round).await?;
            let existing_certs = {
                let datastore = self.datastore.lock().await;
                BlockMessage::find_all_in_round_of_type(&datastore, round - 1, "certified").await?
            };

            for block_message in existing_certs {
                let block_content = block_message.content.clone();
                let block = Block::create_from_json(block_content)?;
                {
                    let datastore = self.datastore.lock().await;
                    datastore.delete(&block.get_id()).await?;
                }
                self.on_receive_certified_block(&block).await?;
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
        {
            let datastore = self.datastore.lock().await;
            datastore
                .set_current_round(newest_uncertified_round)
                .await?;
        }
        Ok(())
    }

    pub async fn get_or_fetch_prev_round_certs(
        &self,
        round: u64,
    ) -> Result<HashMap<String, String>> {
        if round == 0 {
            return Ok(HashMap::new());
        }

        let prev_round = round - 1;
        let mut prev_round_certs = {
            let datastore = self.datastore.lock().await;
            datastore.get_timely_certs_at_round(prev_round).await?
        };
        let prev_round_scribes = self.get_scribes_at_round_id(prev_round).await?;
        let threshold = self.consensus_threshold_at_round_id(prev_round).await?;

        if prev_round_certs.len() as u64 >= threshold {
            return Ok(prev_round_certs);
        }

        if let Some(communication) = &self.communication {
            for peer_id in &prev_round_scribes {
                // Changed round to prev_round in fetch call
                let mut block_data = communication
                    .lock()
                    .await
                    .fetch_scribe_round_certified_block(
                        self.peerid.as_ref().unwrap(),
                        peer_id,
                        peer_id,
                        prev_round, // <-- This was the issue
                    )
                    .await?;

                if block_data.is_none() {
                    for alt_peer_id in &prev_round_scribes {
                        // Also changed here
                        block_data = communication
                            .lock()
                            .await
                            .fetch_scribe_round_certified_block(
                                self.peerid.as_ref().unwrap(),
                                alt_peer_id,
                                peer_id,
                                prev_round, // <-- And here
                            )
                            .await?;

                        if block_data.is_some() {
                            break;
                        }
                    }
                }

                if let Some(block_data) = block_data {
                    let block = Block::from_json_object(block_data.to_json_object()?)?;
                    if block.validate_cert(threshold as usize)? {
                        {
                            let datastore = self.datastore.lock().await;
                            block.save(&datastore).await?;
                        }
                    }
                }
            }
        }

        let prev_round_certs = {
            let datastore = self.datastore.lock().await;
            datastore.get_timely_certs_at_round(prev_round).await?
        };
        Ok(prev_round_certs)
    }

    pub async fn run_round(&mut self, signal: Option<CancellationToken>) -> Result<()> {
        self.speed_up_to_latest_uncertified_round().await?;
        let mut round = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };

        let mut working_round = round;

        loop {
            if working_round < 1 {
                break;
            }

            let prev_round_certs = self.get_or_fetch_prev_round_certs(working_round).await?;
            let threshold = self
                .consensus_threshold_at_round_id(working_round - 1)
                .await?;
            let cert_count = prev_round_certs.len() as u64;

            if cert_count >= threshold {
                break;
            } else {
                warn!(
                    "NOT ENOUGH {}/{} going back to round {}",
                    cert_count,
                    threshold,
                    working_round - 1
                );
                working_round -= 1;
            }
        }

        let prev_round_certs = self.get_or_fetch_prev_round_certs(round).await?;
        let threshold = if round == 0 {
            0
        } else {
            self.consensus_threshold_at_round_id(round - 1).await?
        };
        let cert_count = prev_round_certs.len() as u64;

        if cert_count < threshold {
            warn!(
                "prev_round: {}, cert_count: {}, threshold: {}, prev_round_certs: {:?}",
                round - 1,
                cert_count,
                threshold,
                prev_round_certs
            );
            return Err(anyhow::anyhow!("not enough certs to start round"));
        }

        let current_round_threshold = self.consensus_threshold_at_round_id(round).await?;
        let existing_this_round_certs = {
            let datastore = self.datastore.lock().await;
            BlockMessage::find_all_in_round_of_type(&datastore, round, "certified").await?
        };

        if existing_this_round_certs.len() as u64 >= current_round_threshold {
            self.bump_current_round().await?;
            round = {
                let datastore = self.datastore.lock().await;
                datastore.get_current_round().await?
            };
        }

        let mut cc_events = {
            let datastore = self.datastore.lock().await;
            Transaction::find_all(&datastore).await?
        };
        let keep_waiting_for_events = cc_events.is_empty();

        if keep_waiting_for_events {
            if let Some(wait_time) = self.no_events_round_wait_time_ms {
                tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
            }
        }

        loop {
            if let Some(wait_time) = self.no_events_poll_wait_time_ms {
                tokio::task::yield_now().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(wait_time)).await;
            }
            cc_events = {
                let datastore = self.datastore.lock().await;
                Transaction::find_all(&datastore).await?
            };
            if true || !cc_events.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }

        let mut events = Vec::new();
        for cc_event in cc_events {
            events.push(serde_json::json!({
                "contract_id": cc_event.contract_id,
                "commit_id": cc_event.commit_id,
            }));
            {
                let datastore = self.datastore.lock().await;
                cc_event.delete(&datastore).await?;
            }
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
        {
            let datastore = self.datastore.lock().await;
            block.save(&datastore).await?;
        }

        if let Some(communication) = &self.communication {
            let block_data = block.clone();
            // Take communication lock once
            let mut comm = communication.lock().await;
            comm.broadcast_draft_block(&self.peerid.clone().unwrap(), &block_data)
                .await?;
            // Lock is released here
        }

        // Handle enqueued round messages
        let existing_drafts = {
            let datastore = self.datastore.lock().await;
            BlockMessage::find_all_in_round_of_type(&datastore, round, "draft").await?
        };

        for draft in existing_drafts {
            let draft_content = draft.content.clone();
            let block = Block::create_from_json(draft_content)?;
            {
                let datastore = self.datastore.lock().await;
                datastore.delete(&draft.get_id()).await?
            }
            self.on_receive_draft_block(&block).await?;
        }

        let mut keep_waiting_for_acks = self.latest_seen_at_block_id.is_none();
        let mut keep_waiting_for_certs = true;

        // Outside the loop, spawn tasks to monitor acks and certs
        let mut ack_monitor = {
            let peerid = self.peerid.clone();
            let communication = self.communication.clone();
            let keypair = self.keypair.clone();
            let block = block.clone();
            let threshold = current_round_threshold;
            let datastore_mutex = self.datastore.clone();

            tokio::spawn(async move {
                loop {
                    let mut block_clone = block.clone();
                    {
                        let ds_lock = datastore_mutex.lock().await;
                        if let Err(_) = block_clone.reload(&*ds_lock).await {
                            break;
                        }
                    }
                    let valid_acks = block_clone.count_valid_acks()?;

                    if valid_acks >= (threshold as usize) {
                        if let Some(keypair) = &keypair {
                            block_clone.generate_cert(keypair)?;
                        }
                        {
                            let datastore = datastore_mutex.lock().await;
                            block_clone.save(&datastore).await?;
                        }

                        if let Some(communication) = &communication {
                            let mut comm = communication.lock().await;
                            comm.broadcast_certified_block(
                                &peerid.clone().unwrap(),
                                &block_clone.clone(),
                            )
                            .await?;
                        }
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                Ok::<_, anyhow::Error>(())
            })
        };

        let mut cert_monitor = {
            let datastore = self.datastore.clone();
            let round = round;
            let threshold = current_round_threshold;

            tokio::spawn(async move {
                loop {
                    let current_round_certs = {
                        let datastore = datastore.lock().await;
                        datastore.get_timely_certs_at_round(round).await?
                    };
                    if current_round_certs.len() as u64 >= threshold {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                Ok::<_, anyhow::Error>(())
            })
        };

        // Wait for either condition to be met or latest_seen to change
        loop {
            if let Some(latest_seen) = self.latest_seen_at_block_id {
                if latest_seen > round {
                    // Cancel our monitors
                    tokio::spawn(async move {
                        ack_monitor.abort();
                        cert_monitor.abort();
                    });

                    self.jump_to_round(latest_seen).await?;
                    self.latest_seen_at_block_id = None;
                    return Ok(());
                }
            }

            if signal.as_ref().map_or(false, |s| s.is_cancelled()) {
                if !ack_monitor.is_finished() {
                    ack_monitor.abort();
                }
                if !cert_monitor.is_finished() {
                    cert_monitor.abort();
                }
                return Err(anyhow::anyhow!("aborted"));
            }

            if !keep_waiting_for_acks && !keep_waiting_for_certs {
                break;
            }

            tokio::select! {
                ack_result = &mut ack_monitor, if keep_waiting_for_acks => {
                    ack_result??;
                    keep_waiting_for_acks = false;
                }
                cert_result = &mut cert_monitor, if keep_waiting_for_certs => {
                    cert_result??;
                    keep_waiting_for_certs = false;
                }
                _ = tokio::time::sleep(Duration::from_millis(
                    self.intra_round_wait_time_ms.unwrap_or(50)
                )) => {}
            }
        }

        self.bump_current_round().await?;
        Ok(())
    }
    pub async fn jump_to_round(&mut self, round_num: u64) -> Result<()> {
        let current_round_num = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };
        for _i in (current_round_num + 1)..round_num {
            // TODO: Maybe handle jumping from earlier rounds
        }
        {
            let datastore = self.datastore.lock().await;
            datastore.set_current_round(round_num).await?;
        }
        Ok(())
    }

    pub async fn bump_current_round(&mut self) -> Result<()> {
        {
            let datastore = self.datastore.lock().await;
            datastore.bump_current_round().await?;
        }
        Ok(())
    }

    pub async fn run_until_round(
        &mut self,
        target_round: u64,
        signal: Option<CancellationToken>,
    ) -> Result<()> {
        let mut current_round = {
            let datastore = self.datastore.lock().await;
            datastore.get_current_round().await?
        };
        while current_round < target_round {
            if signal.as_ref().map_or(false, |s| s.is_cancelled()) {
                return Err(anyhow::anyhow!("aborted"));
            }
            self.run_round(signal.clone()).await?;
            current_round = {
                let datastore = self.datastore.lock().await;
                datastore.get_current_round().await?
            };
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
    pub datastore: Arc<Mutex<NetworkDatastore>>,
    pub peerid: Option<String>,
    pub keypair: Option<Keypair>,
    pub communication: Option<Arc<Mutex<dyn Communication>>>,
    pub sequencing: Arc<dyn Sequencing>,
}

pub async fn create_runner_props_from_datastore(
    datastore: Arc<Mutex<NetworkDatastore>>,
) -> Result<RunnerProps> {
    // TODO more than StaticAuthority
    let blocks = {
        let datastore = datastore.lock().await;
        Block::find_all_in_round(&datastore, 0).await?
    };
    let scribes: Vec<String> = blocks.iter().map(|b| b.peer_id.to_string()).collect();
    let election = election::Election::RoundRobin(election::round_robin::RoundRobin::create());
    let sequencing = StaticAuthority::create(scribes.clone(), election).await;

    let runner_props = RunnerProps {
        datastore: datastore,
        peerid: None,
        keypair: None,
        sequencing: Arc::new(sequencing),
        communication: None,
    };
    Ok(runner_props)
}
