import ConsensusMath from "../lib/ConsensusMath.js";
import RoundBlockHeader from "@modality-dev/network-datastore/data/RoundBlockHeader";
export default class StaticAuthority {
  static async create({ datastore, scribes, election }) {
    const sa = new StaticAuthority();
    sa.datastore = datastore;
    sa.election = election;
    sa.scribes = scribes;
    return sa;
  }

  async getScribesAtRound(round) {
    if (!this.scribes) {
      const rbhs = await RoundBlockHeader.findAllInRound({ datastore: this.datastore, round_id: 0 });
      const peer_ids = rbhs.map(i => i.peer_id);
      this.scribes = peer_ids;
    }
    return this.scribes;
  }

  async consensusThresholdForRound(round) {
    return ConsensusMath.calculate2fplus1(this.scribes.length);
  }
}