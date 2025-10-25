use anyhow::Result;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use libp2p_identity::PeerId;

use modality_network_consensus::communication::Communication;
use modal_datastore::models::block::Ack;
use modal_datastore::models::block::Block;
use modality_network_consensus::communication::Message as ConsensusMessage;

use crate::gossip::consensus::block::cert::TOPIC as BLOCK_CERT_TOPIC;
use crate::gossip::consensus::block::draft::TOPIC as BLOCK_DRAFT_TOPIC;
use crate::node::Node;

pub struct NodeCommunication {
    pub swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    pub consensus_tx: mpsc::Sender<ConsensusMessage>,
}

#[async_trait::async_trait]
impl Communication for NodeCommunication {
    async fn broadcast_draft_block(&mut self, from_peer: &str, block: &Block) -> Result<()> {
        let msg = ConsensusMessage::DraftBlock {
            from: from_peer.to_string(),
            to: String::new(),
            block: block.clone(),
          };
        self.consensus_tx.send(msg).await?;
        {
            let mut swarm = self.swarm.lock().await;
            swarm.behaviour_mut().gossipsub.publish(
                libp2p::gossipsub::IdentTopic::new(BLOCK_DRAFT_TOPIC),
                serde_json::to_string(block)?,
            )?;
        }
        Ok(())
    }

    async fn broadcast_certified_block(&mut self, from_peer: &str, block: &Block) -> Result<()> {
        let msg = ConsensusMessage::CertifiedBlock {
            from: from_peer.to_string(),
            to: String::new(),
            block: block.clone(),
          };
        self.consensus_tx.send(msg).await?;
        {
            let mut swarm = self.swarm.lock().await;
            swarm.behaviour_mut().gossipsub.publish(
                libp2p::gossipsub::IdentTopic::new(BLOCK_CERT_TOPIC),
                serde_json::to_string(block)?,
            )?;
        }
        Ok(())
    }

    async fn send_block_ack(&mut self, from_peer: &str, to_peer: &str, ack: &Ack) -> Result<()> {
        let target_peer = PeerId::from_str(to_peer)?;
        let request = crate::reqres::Request {
            path: "/consensus/block/ack".into(),
            data: Some(serde_json::json!(ack)),
        };
        if ack.peer_id == ack.acker {
            let msg = ConsensusMessage::BlockAck {
                from: from_peer.to_string(),
                to: String::new(),
                ack: ack.clone(),
              };
            self.consensus_tx.send(msg).await?;
        } else {
            let mut swarm = self.swarm.lock().await;
            let _req_id = swarm
                .behaviour_mut()
                .reqres
                .send_request(&target_peer, request);
        }

        Ok(())
    }

    async fn send_block_late_ack(
        &mut self,
        from_peer: &str,
        to_peer: &str,
        ack: &Ack,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn fetch_scribe_round_certified_block(
        &mut self,
        from_peer: &str,
        to_peer: &str,
        scribe_peer: &str,
        round: u64,
    ) -> Result<Option<Block>> {
        Ok(None)
        // let target_peer = PeerId::from_str(to_peer)?;
        // let request = crate::reqres::Request {
        //     path: "fetch_certified_block".into(),
        //     data: Some(serde_json::json!({
        //         "scribe_peer": scribe_peer,
        //         "round": round
        //     })),
        // };

        // let mut swarm = self.swarm.lock().await;
        // let req_id = swarm.behaviour_mut().reqres.send_request(&target_peer, request);

        // // Wait for response
        // loop {
        //     match swarm.select_next_some().await {
        //         SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
        //             request_response::Event::Message {
        //                 message: request_response::Message::Response { response, request_id, .. },
        //                 ..
        //             }
        //         )) => {
        //             if req_id == request_id {
        //                 if response.data.is_some() {
        //                     return Ok(serde_json::from_value(response.data.unwrap())?);
        //                 } else {
        //                     return Ok(None);
        //                 }
        //             }
        //         }
        //         _ => {}
        //     }
        // }
    }
}
