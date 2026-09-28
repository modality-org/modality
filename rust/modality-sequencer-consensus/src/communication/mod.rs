use anyhow::Result;

use modality_datastore::models::sequencer::block::SequencerBlock;
use modality_datastore::models::sequencer::block::Ack;

#[async_trait::async_trait]
pub trait Communication: Send + Sync {
    async fn broadcast_draft_block(&mut self, from: &str, block_data: &SequencerBlock) -> Result<()>;
    async fn broadcast_certified_block(&mut self, from: &str, block_data: &SequencerBlock) -> Result<()>;
    async fn send_block_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn send_block_late_ack(&mut self, from: &str, to: &str, ack_data: &Ack) -> Result<()>;
    async fn fetch_scribe_round_certified_block(&mut self, from: &str, to: &str, peer_id: &str, round_id: u64) -> Result<Option<SequencerBlock>>;
}


// Message types that can be sent through the channel
#[derive(Debug)]
pub enum Message {
    DraftSequencerBlock {
        #[allow(dead_code)]
        from: String,
        to: String,
        block: SequencerBlock,
    },
    SequencerBlockAck {
        #[allow(dead_code)]
        from: String,
        to: String,
        ack: Ack,
    },
    SequencerBlockLateAck {
        #[allow(dead_code)]
        from: String,
        to: String,
        ack: Ack,
    },
    CertifiedSequencerBlock {
        #[allow(dead_code)]
        from: String,
        to: String,
        block: SequencerBlock,
    },
}