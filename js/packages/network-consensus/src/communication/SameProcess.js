export default class SameProcess {
  constructor({ nodes } = {}) {
    this.nodes = nodes;
    this.offline_nodes = [];
  }

  async broadcastDraftPage({ from, page_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    for (const to_seq of Object.values(this.nodes)) {
      if (this.offline_nodes.includes(to_seq.peerid)) {
        continue;
      }
      await to_seq?.onReceiveDraftPage(page_data);
    }
  }

  async sendPageAck({ from, to, ack_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    if (this.offline_nodes.includes(to)) {
      return;
    }
    const to_seq = this.nodes[to];
    await to_seq?.onReceivePageAck(ack_data);
  }

  async sendPageLateAck({ from, to, ack_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    if (this.offline_nodes.includes(to)) {
      return;
    }
    const to_seq = this.nodes[to];
    await to_seq?.onReceivePageLateAck(ack_data);
  }

  async broadcastCertifiedPage({ from, page_data }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    for (const to_seq of Object.values(this.nodes)) {
      if (this.offline_nodes.includes(to_seq.peerid)) {
        continue;
      }
      await to_seq?.onReceiveCertifiedPage(page_data);
    }
  }

  async fetchScribeRoundCertifiedPage({ from, to, scribe, round }) {
    if (this.offline_nodes.includes(from)) {
      return;
    }
    const to_seq = this.nodes[to];
    return to_seq?.onFetchScribeRoundCertifiedPageRequest({scribe, round});
  };
}
