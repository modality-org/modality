use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::communication::Communication;

use modality_network_datastore::models::page::Page;
use modality_network_datastore::models::page::Ack;

pub struct SameProcess {
    nodes: HashMap<String, Arc<dyn Node>>,
    offline_nodes: Vec<String>,
}

#[async_trait]
pub trait Node: Send + Sync {
    fn peerid(&self) -> &str;
    async fn on_receive_draft_page(&self, page_data: &Page) -> Result<()>;
    async fn on_receive_page_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_page_late_ack(&self, ack_data: &Ack) -> Result<()>;
    async fn on_receive_certified_page(&self, page_data: &Page) -> Result<()>;
    async fn on_fetch_scribe_round_certified_page_request(&self, scribe: &str, round: u64) -> Result<Option<Page>>;
}

impl SameProcess {
    pub fn new(nodes: HashMap<String, Arc<dyn Node>>) -> Self {
        Self {
            nodes,
            offline_nodes: Vec::new(),
        }
    }
}

#[async_trait]
impl Communication for SameProcess {
    async fn broadcast_draft_page(&self, from: &str, page_data: &Page) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if self.offline_nodes.contains(&node.peerid().to_string()) {
                continue;
            }
            node.on_receive_draft_page(page_data).await?;
        }
        Ok(())
    }

    async fn send_page_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) || 
           self.offline_nodes.contains(&to.to_string()) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_page_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn send_page_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) || 
           self.offline_nodes.contains(&to.to_string()) {
            return Ok(());
        }

        if let Some(node) = self.nodes.get(to) {
            node.on_receive_page_late_ack(ack_data).await?;
        }
        Ok(())
    }

    async fn broadcast_certified_page(&self, from: &str, page_data: &Page) -> Result<()> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(());
        }

        for node in self.nodes.values() {
            if self.offline_nodes.contains(&node.peerid().to_string()) {
                continue;
            }
            node.on_receive_certified_page(page_data).await?;
        }
        Ok(())
    }

    async fn fetch_scribe_round_certified_page(&self, from: &str, to: &str, scribe: &str, round: u64) -> Result<Option<Page>> {
        if self.offline_nodes.contains(&from.to_string()) {
            return Ok(None);
        }

        if let Some(node) = self.nodes.get(to) {
            return node.on_fetch_scribe_round_certified_page_request(scribe, round).await;
        }
        Ok(None)
    }
}