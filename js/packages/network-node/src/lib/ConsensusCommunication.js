import { ICommunication } from '@modality-dev/network-consensus/communication/ICommunication';

import { TOPIC as PAGE_DRAFT_TOPIC } from "../gossip/consensus/block/draft.js";
import { TOPIC as PAGE_CERT_TOPIC } from "../gossip/consensus/block/cert.js";

/**
 * @implements {ICommunication}
 */
export default class ConsensusCommunication {
  constructor({ node }) {
    this.node = node;
  }

  async broadcastDraftBlock({ from, block_data }) {
    await this.node.publishGossip(PAGE_DRAFT_TOPIC, block_data);
  }

  async sendBlockAck({ from, to, ack_data }) {
    return await this.node.sendOrHandleRequest(
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
