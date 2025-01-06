import ConsensusMath from "../lib/ConsensusMath";

export default class StaticAuthority {
  static async create({scribes, election}) {
    const sa = new StaticAuthority();
    sa.election = election;
    sa.scribes = scribes;
    return sa;
  }

  async getScribesAtRound(round) {
    return this.scribes;
  }

  async consensusThresholdForRound(round) {
    return ConsensusMath.calculate2fplus1(this.scribes.length);
  }
}