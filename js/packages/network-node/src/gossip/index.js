import * as ConsensusScribesPageDraft from "../gossip/consensus/scribes/page_draft.js";
import * as ConsensusScribesPageCert from "../gossip/consensus/scribes/page_cert.js";

export const SEQUENCER_TOPIC_MODULES = [
  ConsensusScribesPageDraft,
  ConsensusScribesPageCert,
];

export async function addSequencerEventListeners(node) {
  for (const module of SEQUENCER_TOPIC_MODULES) {
    // console.log("PUBSUB SUBSCRIBE", node.peerId, module.TOPIC);
    node.services.pubsub.subscribe(module.TOPIC);
  }
  node.services.pubsub.addEventListener("message", (message) => {
    // console.log("PUBSUB MESSAGE", node.peerId, message.detail.topic);
    const topic = message.detail.topic;
    for (const module of SEQUENCER_TOPIC_MODULES) {
      if (topic === module.TOPIC) {
        module.handler(node, message);
      }
    }
  });
}
