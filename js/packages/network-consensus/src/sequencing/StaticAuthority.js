import ConsensusMath from "../lib/ConsensusMath.js";
import BlockHeader from "@modality-dev/network-datastore/data/BlockHeader";
export default class StaticAuthority {
  static async create({ datastore, scribes, election }) {
    const sa = new StaticAuthority();
    sa.datastore = datastore;
    sa.election = election;
    sa.scribes = scribes;
    return sa;
  }

  async getScribesAtRound(round) {
    if (round < 0) {
      return [];
    }
    if (!this.scribes) {
      const block_headers = await BlockHeader.findAllInRound({
        datastore: this.datastore,
        round_id: 0,
      });
      const peer_ids = block_headers.map((i) => i.peer_id);
      this.scribes = peer_ids;
    }
    return this.scribes;
  }

  async consensusThresholdForRound(round) {
    if (round < 0) {
      return 0;
    }
    return ConsensusMath.calculate2fplus1(this.scribes.length);
  }
}
