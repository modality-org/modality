pub mod same_process;

use anyhow::Result;

use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

#[async_trait::async_trait]
pub trait Communication: Send + Sync {
    async fn broadcast_draft_block(&self, from: &str, block_data: &Block) -> Result<()>;
    async fn broadcast_certified_page(&self, from: &str, block_data: &Block) -> Result<()>;
    async fn send_block_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn send_block_late_ack(&self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn fetch_peer_block_certified_page(&self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>>;
}
