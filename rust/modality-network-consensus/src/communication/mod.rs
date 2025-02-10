pub mod same_process;

use anyhow::Result;

use modality_network_datastore::models::block::Block;
use modality_network_datastore::models::block::Ack;

#[async_trait::async_trait]
pub trait Communication: Send + Sync {
    async fn broadcast_draft_block(&mut self, from: &str, block_data: &Block) -> Result<()>;
    async fn broadcast_certified_block(&mut self, from: &str, block_data: &Block) -> Result<()>;
    async fn send_block_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn send_block_late_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn fetch_scribe_round_certified_block(&mut self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<Block>>;
}
