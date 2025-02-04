export default class SameProcess {
  constructor({ nodes } = {}) {
    this.nodes = nodes;
    this.offline_nodes = [];
  }

  async broadcastDraftBlock({ from, block_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    for (const to_seq of Object.values(this.nodes)) {
      if (this.offline_nodes.includes(to_seq.peerid)) {
        continue;
      }
      await to_seq?.onReceiveBlockDraft(block_data);
    }
  }

  async sendBlockAck({ from, to, ack_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    if (this.offline_nodes.includes(to)) {
      return;
    }
    const to_seq = this.nodes[to];
    await to_seq?.onReceiveBlockAck(ack_data);
  }

  async sendBlockLateAck({ from, to, ack_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    if (this.offline_nodes.includes(to)) {
      return;
    }
    const to_seq = this.nodes[to];
    await to_seq?.onReceiveBlockLateAck(ack_data);
  }

  async broadcastCertifiedBlock({ from, block_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    for (const to_seq of Object.values(this.nodes)) {
      if (this.offline_nodes.includes(to_seq.peerid)) {
        continue;
      }
      await to_seq?.onReceiveBlockCert(block_data);
    }
  }

  async fetchScribeRoundCertifiedBlock({ from, to, scribe, round }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    const to_seq = this.nodes[to];
    return to_seq?.onFetchScribeRoundCertifiedBlockRequest({scribe, round});
  };
}
