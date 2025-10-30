pub mod static_authority;

use anyhow::Result;

#[async_trait::async_trait]
pub trait Sequencing: Send + Sync {
    async fn get_scribes_at_round_id(&self, round: u64) -> Result<Vec<String>>;
    async fn consensus_threshold_at_round_id(&self, round: u64) -> Result<u64>;
}