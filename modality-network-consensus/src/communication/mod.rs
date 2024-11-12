pub mod same_process;

use anyhow::Result;

use modality_network_datastore::models::page::Page;
use modality_network_datastore::models::page::Ack;

#[async_trait::async_trait]
pub trait Communication: Send + Sync {
    async fn broadcast_draft_page(&self, from: &str, page_data: &Page) -> Result<()>;
    async fn broadcast_certified_page(&self, from: &str, page_data: &Page) -> Result<()>;
    async fn send_page_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn send_page_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn fetch_scribe_round_certified_page(&self, from: &str, to: &str, scribe: &str, round: u64) -> Result<Option<Page>>;
}
