import { peerIdFromString } from '@libp2p/peer-id'

import { TOPIC as PAGE_DRAFT_TOPIC } from "../gossip/consensus/scribes/page_draft.js";
import { TOPIC as PAGE_CERT_TOPIC } from "../gossip/consensus/scribes/page_cert.js";

export default class ConsensusCommunication {
  constructor({ node, sequencer }) {
    this.node = node;
    this.sequencer = sequencer;
    return this;
  }

  async callReqres( to, path, data ) {
    if (to === this.node.peerId.toString()) {
      return await this.node.services.reqres.handleRequest(this.node.peerId, path, data, {node: this.node});
    } else {
      return await this.node.services.reqres.call(
        peerIdFromString(to),
        path,
        data,
        {node: this.node}
      );
    }
  }

  async broadcastDraftPage({ from, page_data }) {
    const json_text = new TextEncoder().encode(JSON.stringify(page_data))
    await this.node.services.pubsub.publish(PAGE_DRAFT_TOPIC, json_text);
  }

  async sendPageAck({ from, to, ack_data }) {
    return await this.callReqres(
      to,
      "/consensus/scribes/page_ack",
      ack_data
    );
  }

  async sendPageLateAck({ from, to, ack_data }) {
    // not implemented
  }

  async broadcastCertifiedPage({ from, page_data }) {
    const json_text = new TextEncoder().encode(JSON.stringify(page_data))
    await this.node.services.pubsub.publish(PAGE_CERT_TOPIC, json_text);
  }

  async fetchScribeRoundCertifiedPage({ from, to, scribe, round }) {
    if (to === this.node.peerId.toString()) { return null; }
    return await this.callReqres(
      to,
      "/data/scribe_round_page",
      { scribe, round }
    );
  }
}
