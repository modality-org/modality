import { peerIdFromString } from '@libp2p/peer-id'

import { TOPIC as PAGE_DRAFT_TOPIC } from "../gossip/consensus/block/draft.js";
import { TOPIC as PAGE_CERT_TOPIC } from "../gossip/consensus/block/cert.js";

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

  async broadcastDraftBlock({ from, block_data }) {
    await this.node.publishGossip(PAGE_DRAFT_TOPIC, block_data);
  }

  async sendBlockAck({ from, to, ack_data }) {
    return await this.sendRequest(
      to,
      "/consensus/block/ack",
      ack_data
    );
  }

  async sendBlockLateAck({ from, to, ack_data }) {
    // not implemented
  }

  async broadcastCertifiedBlock({ from, block_data }) {
    await this.node.publishGossip(PAGE_CERT_TOPIC, block_data);
  }

  async fetchScribeRoundCertifiedBlock({ from, to, round_id, peer_id }) {
    if (to === this.node.peerid) { return null; }
    return await this.node.sendRequest(
      to,
      "/data/block",
      { round_id, peer_id }
    );
  }
}
