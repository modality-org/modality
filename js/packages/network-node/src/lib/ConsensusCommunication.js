import { peerIdFromString } from '@libp2p/peer-id'

import { TOPIC as PAGE_DRAFT_TOPIC } from "../gossip/consensus/scribes/page_draft.js";
import { TOPIC as PAGE_CERT_TOPIC } from "../gossip/consensus/scribes/page_cert.js";

export default class ConsensusCommunication {
  constructor({ node }) {
    this.node = node;
    return this;
  }

  async sendRequest( to, path, data ) {
    if (to === this.node.peerid) {
      return await this.node.handleRequest(
        this.node.peerid,
        path,
        data
      );
    } else {
      return await this.node.sendRequest(
        to,
        path,
        data,
      );
    }
  }

  async broadcastDraftPage({ from, page_data }) {
    await this.node.publishGossip(PAGE_DRAFT_TOPIC, page_data);
  }

  async sendPageAck({ from, to, ack_data }) {
    return await this.sendRequest(
      to,
      "/consensus/scribes/page_ack",
      ack_data
    );
  }

  async sendPageLateAck({ from, to, ack_data }) {
    // not implemented
  }

  async broadcastCertifiedPage({ from, page_data }) {
    await this.node.publishGossip(PAGE_CERT_TOPIC, page_data);
  }

  async fetchScribeRoundCertifiedPage({ from, to, scribe, round }) {
    if (to === this.node.peerid) { return null; }
    return await this.node.sendRequest(
      to,
      "/data/scribe_round_page",
      { scribe, round }
    );
  }
}
